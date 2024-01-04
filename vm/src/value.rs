use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use core_types::interner::Interned;

use crate::{Heap, VMResult};
use crate::vm::GVm;

pub type CallFuncSig<ENV> = fn(vm: &mut GVm<ENV>, registers: &[Value]) -> VMResult<Value>;
#[derive(Copy, Clone)]
pub struct CallFunc<ENV> {
    pub func: CallFuncSig<ENV>,
}

impl<ENV> PartialEq for CallFunc<ENV> {
    fn eq(&self, other: &CallFunc<ENV>) -> bool {
        std::ptr::eq(
            self.func as *const CallFuncSig<ENV>,
            other.func as *const CallFuncSig<ENV>,
        )
    }
}

impl<ENV> Eq for CallFunc<ENV> {}

impl<ENV> Hash for CallFunc<ENV> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.func as usize);
    }
}

impl<ENV> fmt::Debug for CallFunc<ENV> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "...")
    }
}

pub struct PairIter<'vm, ENV> {
    vm: &'vm GVm<ENV>,
    current: Option<Value>,
    dotted: bool,
}

impl<'vm, ENV> PairIter<'vm, ENV> {
    pub fn new(vm: &'vm GVm<ENV>, exp: Value) -> Self {
        Self {
            vm,
            current: Some(exp),
            dotted: false,
        }
    }

    pub fn is_dotted(&self) -> bool {
        self.dotted
    }
}

impl<'vm, ENV> Iterator for PairIter<'vm, ENV> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            match current {
                Value::Pair(h) => {
                    let (car, cdr) = self.vm.get_pair(h);
                    self.current = Some(cdr);
                    Some(car)
                }
                // TODO: Handle List?
                Value::Nil => None,
                _ => {
                    let cur = Some(current);
                    self.current = None;
                    self.dotted = true;
                    cur
                }
            }
        } else {
            None
        }
    }
}

pub const INT_BITS: u8 = 56;
pub const INT_MAX: i64 = 2_i64.pow(INT_BITS as u32 - 1) - 1;
pub const INT_MIN: i64 = -(2_i64.pow(INT_BITS as u32 - 1));

pub fn from_i56(arr: &[u8; 7]) -> i64 {
    let mut bytes = [0x00, arr[0], arr[1], arr[2], arr[3], arr[4], arr[5], arr[6]];
    if (arr[0] & 0x80) > 0 {
        bytes[0] = 0xff;
        i64::from_be_bytes(bytes)
    } else {
        i64::from_be_bytes(bytes)
    }
}

pub fn to_i56(i: i64) -> Value {
    let bytes = i.to_be_bytes();
    let bytes7 = [
        bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ];
    Value::Int(bytes7)
}

#[derive(Clone, Debug)]
pub struct Globals {
    objects: Vec<Value>,
    props: HashMap<u32, Arc<HashMap<Interned, Value>>>,
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}

impl Globals {
    pub fn new() -> Self {
        Globals {
            objects: Vec::new(),
            props: HashMap::new(),
        }
    }

    pub fn reserve(&mut self) -> u32 {
        let index = self.objects.len();
        self.objects.push(Value::Undefined);
        index as u32
    }

    /// Sets a global to val.  The value needs have local numbers promoted to the heap before
    /// setting it.
    pub fn set(&mut self, idx: u32, val: Value) {
        self.objects[idx as usize] = val;
    }

    pub fn get(&self, idx: u32) -> Value {
        self.objects
            .get(idx as usize)
            .map_or_else(|| Value::Undefined, |v| *v)
    }

    pub fn mark(&self, heap: &mut Heap) {
        self.objects.iter().for_each(|obj| {
            heap.mark(*obj);
        });
        self.props.iter().for_each(|(_, map)| {
            for val in map.values() {
                heap.mark(*val);
            }
        });
    }

    pub fn get_property(&self, global: u32, prop: Interned) -> Option<Value> {
        if let Some(map) = self.props.get(&global) {
            if let Some(val) = map.get(&prop) {
                return Some(*val);
            }
        }
        None
    }

    pub fn set_property(&mut self, global: u32, prop: Interned, value: Value) {
        if let Some(map) = self.props.get_mut(&global) {
            let map = Arc::make_mut(map);
            map.insert(prop, value);
        } else {
            let mut map = HashMap::new();
            map.insert(prop, value);
            self.props.insert(global, Arc::new(map));
        }
    }
}
