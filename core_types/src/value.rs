use std::hash::{Hash, Hasher};
use slvm::error::{VMError, VMResult};
use slvm::value;
use slvm::value::PairIter;
use slvm::vm::GVm;
use std::iter;

// Do this wrap nonsense so that Value is hashable...
#[derive(Copy, Clone, Debug)]
pub struct F32Wrap(pub f32);

impl PartialEq for F32Wrap {
    fn eq(&self, other: &Self) -> bool {
        (self.0 - other.0).abs() < f32::EPSILON
        //self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for F32Wrap {}

impl Hash for F32Wrap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.0.to_bits());
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Value {
    Byte(u8),
    Int([u8; 7]), // Store a 7 byte int (i56...).
    Float(F32Wrap),
    CodePoint(char),
    CharCluster(u8, [u8; 6]),
    CharClusterLong(Handle), // Handle points to a String on the heap.
    Symbol(Interned),
    Keyword(Interned),
    StringConst(Interned),
    Special(Interned), // Intended for symbols that are compiled.
    Builtin(u32),
    True,
    False,
    Nil,
    Undefined,

    String(Handle),
    Vector(Handle),
    Map(Handle),
    Bytes(Handle),
    Pair(Handle),
    List(Handle, u16),
    Lambda(Handle),
    Closure(Handle),
    Continuation(Handle),
    CallFrame(Handle),
    Value(Handle),
    Error(Handle),
}

impl Default for Value {
    fn default() -> Self {
        Self::new()
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float(F32Wrap(value))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(F32Wrap(value as f32))
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        value::to_i56(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        value::to_i56(value as i64)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        value::to_i56(value as i64)
    }
}

impl Value {
    pub fn new() -> Self {
        Value::Undefined
    }

    #[inline(always)]
    pub fn unref<ENV>(self, vm: &GVm<ENV>) -> Value {
        match &self {
            Value::Value(handle) => vm.get_value(*handle),
            _ => self,
        }
    }

    pub fn get_symbol(&self) -> Option<Interned> {
        if let Value::Symbol(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    pub fn is_symbol(&self, sym: Interned) -> bool {
        if let Value::Symbol(i) = self {
            *i == sym
        } else {
            false
        }
    }

    pub fn is_indirect(&self) -> bool {
        matches!(self, Value::Value(_))
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn is_undef(&self) -> bool {
        matches!(self, Value::Undefined)
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Value::True)
    }

    pub fn is_truethy(&self) -> bool {
        !matches!(self, Value::False | Value::Nil)
    }

    pub fn is_false(&self) -> bool {
        matches!(self, Value::False)
    }

    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::False | Value::Nil)
    }

    pub fn is_int(&self) -> bool {
        matches!(&self, Value::Byte(_) | Value::Int(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(&self, Value::Byte(_) | Value::Int(_) | Value::Float(_))
    }

    pub fn get_int<ENV>(&self, _vm: &GVm<ENV>) -> VMResult<i64> {
        match &self {
            Value::Byte(b) => Ok(*b as i64),
            Value::Int(i) => Ok(value::from_i56(i)),
            _ => Err(VMError::new_value(format!("Not an integer: {self:?}"))),
        }
    }

    pub fn get_float<ENV>(&self, _vm: &GVm<ENV>) -> VMResult<f32> {
        match &self {
            Value::Byte(b) => Ok(*b as f32),
            Value::Int(i) => Ok(value::from_i56(i) as f32),
            Value::Float(f) => Ok(f.0),
            _ => Err(VMError::new_value(format!("Not a float: {self:?}"))),
        }
    }

    pub fn get_string<'vm, ENV>(&self, vm: &'vm GVm<ENV>) -> VMResult<&'vm str> {
        match &self {
            Value::String(h) => Ok(vm.get_string(*h)),
            Value::StringConst(i) => Ok(vm.get_interned(*i)),
            // TODO- handle chars/codepoints...
            _ => Err(VMError::new_value(format!("Not a string: {self:?}"))),
        }
    }

    pub fn get_handle(&self) -> Option<Handle> {
        match &self {
            Value::CharClusterLong(handle) => Some(*handle),
            Value::String(handle) => Some(*handle),
            Value::Vector(handle) => Some(*handle),
            Value::Map(handle) => Some(*handle),
            Value::Bytes(handle) => Some(*handle),
            Value::Pair(handle) => Some(*handle),
            Value::List(handle, _) => Some(*handle),
            Value::Lambda(handle) => Some(*handle),
            Value::Closure(handle) => Some(*handle),
            Value::Continuation(handle) => Some(*handle),
            Value::CallFrame(handle) => Some(*handle),
            Value::Value(handle) => Some(*handle),
            Value::Error(handle) => Some(*handle),

            Value::Byte(_) => None,
            Value::Int(_) => None,
            Value::Float(_) => None,
            Value::CodePoint(_) => None,
            Value::CharCluster(_, _) => None,
            Value::Symbol(_) => None,
            Value::Keyword(_) => None,
            Value::Special(_) => None,
            Value::StringConst(_) => None,
            Value::Builtin(_) => None,
            Value::True => None,
            Value::False => None,
            Value::Nil => None,
            Value::Undefined => None,
        }
    }

    pub fn get_pair<ENV>(&self, vm: &GVm<ENV>) -> Option<(Value, Value)> {
        match &self {
            Value::Pair(handle) => {
                let (car, cdr) = vm.get_pair(*handle);
                Some((car, cdr))
            }
            Value::List(handle, start_u32) => {
                let start = *start_u32 as usize;
                let v = vm.get_vector(*handle);
                let car = if start < v.len() {
                    v[start]
                } else {
                    Value::Nil
                };
                let cdr = if start + 1 < v.len() {
                    Value::List(*handle, start_u32 + 1)
                } else {
                    Value::Nil
                };
                Some((car, cdr))
            }
            _ => None,
        }
    }

    pub fn iter<'vm, ENV>(&self, vm: &'vm GVm<ENV>) -> Box<dyn Iterator<Item = Value> + 'vm> {
        match &self.unref(vm) {
            Value::Pair(_) => Box::new(PairIter::new(vm, *self)),
            Value::List(handle, start) => {
                Box::new(vm.get_vector(*handle)[*start as usize..].iter().copied())
            }
            Value::Vector(handle) => Box::new(vm.get_vector(*handle).iter().copied()),
            _ => Box::new(iter::empty()),
        }
    }

    pub fn display_value<ENV>(&self, vm: &GVm<ENV>) -> String {
        fn list_out_iter<ENV>(
            vm: &GVm<ENV>,
            res: &mut String,
            itr: &mut dyn Iterator<Item = Value>,
        ) {
            let mut first = true;
            for p in itr {
                if !first {
                    res.push(' ');
                } else {
                    first = false;
                }
                res.push_str(&p.display_value(vm));
            }
        }
        fn list_out<ENV>(vm: &GVm<ENV>, res: &mut String, lst: Value) {
            let mut first = true;
            let mut cdr = lst;
            loop {
                if let Value::Nil = cdr {
                    break;
                }
                if !first {
                    res.push(' ');
                } else {
                    first = false;
                }
                match cdr {
                    Value::Pair(handle) => {
                        let (car, ncdr) = vm.get_pair(handle);
                        res.push_str(&car.display_value(vm));
                        cdr = ncdr;
                    }
                    _ => {
                        res.push_str(". ");
                        res.push_str(&cdr.display_value(vm));
                        break;
                    }
                }
            }
        }
        match self {
            Value::True => "true".to_string(),
            Value::False => "false".to_string(),
            Value::Int(i) => format!("{}", value::from_i56(i)),
            Value::Float(f) => format!("{}", f.0),
            Value::Byte(b) => format!("{b}"),
            Value::Symbol(i) => vm.get_interned(*i).to_string(),
            Value::Keyword(i) => format!(":{}", vm.get_interned(*i)),
            Value::StringConst(i) => format!("\"{}\"", vm.get_interned(*i)),
            Value::Special(i) => format!("#<SpecialFn({})>", vm.get_interned(*i)),
            Value::CodePoint(ch) => format!("\\{ch}"),
            Value::CharCluster(l, c) => {
                format!("\\{}", String::from_utf8_lossy(&c[0..*l as usize]))
            }
            Value::CharClusterLong(h) => format!("\\{}", vm.get_string(*h)),
            Value::Builtin(_) => "#<Function>".to_string(),
            Value::Nil => "nil".to_string(),
            Value::Undefined => "#<Undefined>".to_string(), //panic!("Tried to get type for undefined!"),
            Value::Lambda(_) => "#<Lambda>".to_string(),
            Value::Closure(_) => "#<Lambda>".to_string(),
            Value::Continuation(_) => "#<Continuation>".to_string(),
            Value::CallFrame(_) => "#<CallFrame>".to_string(),
            Value::Vector(handle) => {
                let v = vm.get_vector(*handle);
                let mut res = String::new();
                res.push('[');
                list_out_iter(vm, &mut res, &mut v.iter().copied());
                res.push(']');
                res
            }
            Value::Map(handle) => {
                let mut res = String::new();
                res.push('{');
                for (key, val) in vm.get_map(*handle).iter() {
                    res.push_str(&format!(
                        "{} {}\n",
                        key.display_value(vm),
                        val.display_value(vm)
                    ));
                }
                res.push('}');
                res
            }
            Value::Pair(_) => {
                let mut res = String::new();
                res.push('(');
                list_out(vm, &mut res, *self);
                res.push(')');
                res
            }
            Value::List(handle, start) => {
                let v = vm.get_vector(*handle);
                let mut res = String::new();
                res.push('(');
                list_out_iter(vm, &mut res, &mut v[*start as usize..].iter().copied());
                res.push(')');
                res
            }
            Value::String(handle) => format!("\"{}\"", vm.get_string(*handle)),
            Value::Bytes(_) => "Bytes".to_string(), // XXX TODO
            Value::Value(handle) => vm.get_value(*handle).display_value(vm),
            Value::Error(handle) => {
                let err = vm.get_error(*handle);
                let key = vm.get_interned(err.keyword);
                format!("error [{key}]: {}", err.data.display_value(vm))
            }
        }
    }

    pub fn pretty_value<ENV>(&self, vm: &GVm<ENV>) -> String {
        match self {
            Value::StringConst(i) => vm.get_interned(*i).to_string(),
            Value::CodePoint(ch) => format!("{ch}"),
            Value::CharCluster(l, c) => {
                format!("{}", String::from_utf8_lossy(&c[0..*l as usize]))
            }
            Value::CharClusterLong(h) => vm.get_string(*h).to_string(),
            Value::String(handle) => vm.get_string(*handle).to_string(),
            _ => self.display_value(vm),
        }
    }

    pub fn display_type<ENV>(&self, vm: &GVm<ENV>) -> &'static str {
        match self {
            Value::True => "True",
            Value::False => "False",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Symbol(_) => "Symbol",
            Value::Keyword(_) => "Keyword",
            Value::StringConst(_) => "String",
            Value::Special(_) => "Special",
            Value::CodePoint(_) => "Char",
            Value::CharCluster(_, _) => "Char",
            Value::CharClusterLong(_) => "Char",
            Value::Builtin(_) => "Builtin",
            Value::Byte(_) => "Byte",
            Value::Nil => "Nil",
            Value::Undefined => "Undefined", //panic!("Tried to get type for undefined!"),
            Value::Lambda(_) => "Lambda",
            Value::Closure(_) => "Lambda",
            Value::Continuation(_) => "Continuation",
            Value::CallFrame(_) => "CallFrame",
            Value::Vector(_) => "Vector",
            Value::Map(_) => "Map",
            Value::Pair(_) => "Pair",
            Value::List(_, _) => "Pair",
            Value::String(_) => "String",
            Value::Bytes(_) => "Bytes",
            Value::Value(handle) => vm.get_value(*handle).display_type(vm),
            Value::Error(_) => "Error",
        }
    }

    pub fn is_proper_list<ENV>(&self, vm: &GVm<ENV>) -> bool {
        // does not detect empty (nil) lists on purpose.
        if let Value::Pair(handle) = self {
            let (_car, cdr) = vm.get_pair(*handle);
            if cdr.is_nil() {
                true
            } else {
                cdr.is_proper_list(vm)
            }
        } else {
            matches!(self, Value::List(_, _))
        }
    }
}
