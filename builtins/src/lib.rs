use compile_state::state::{SloshVm, SloshVmTrait};
use slvm::{VMError, VMResult, Value};

pub mod collections;
pub mod print;
pub mod string;

pub fn get_prop(vm: &mut SloshVm, registers: &[Value]) -> VMResult<Value> {
    if registers.len() != 2 {
        return Err(VMError::new_vm(
            "get-prop: Invalid arguments (object symbol)".to_string(),
        ));
    }
    let key = match registers[1] {
        Value::Keyword(key) => key,
        Value::Symbol(key) => key,
        _ => return Err(VMError::new_vm("get-prop: key must be a keywork or symbol")),
    };
    match registers[0] {
        Value::Symbol(si) => {
            if let Some(idx) = vm.global_intern_slot(si) {
                Ok(vm.get_global_property(idx, key).unwrap_or(Value::Nil))
            } else {
                Err(VMError::new_vm(
                    "get-prop: Not a heap object or global symbol".to_string(),
                ))
            }
        }
        _ => Ok(vm
            .get_heap_property_interned(registers[0], key)
            .unwrap_or(Value::Nil)),
    }
}

pub fn set_prop(vm: &mut SloshVm, registers: &[Value]) -> VMResult<Value> {
    if registers.len() != 3 {
        return Err(VMError::new_vm(
            "set-prop: Invalid arguments (object symbol value)".to_string(),
        ));
    }
    let key = match registers[1] {
        Value::Keyword(key) => key,
        Value::Symbol(key) => key,
        _ => return Err(VMError::new_vm("set-prop: key must be a keyword or symbol")),
    };
    if let Value::Symbol(si) = registers[0] {
        if let Some(idx) = vm.global_intern_slot(si) {
            vm.set_global_property(idx, key, registers[2]);
            Ok(registers[2])
        } else {
            Err(VMError::new_vm(
                "set-prop: Not a heap object or global symbol".to_string(),
            ))
        }
    } else {
        vm.set_heap_property_interned(registers[0], key, registers[2]);
        Ok(registers[2])
    }
}

pub fn sizeof_heap_object(_vm: &mut SloshVm, registers: &[Value]) -> VMResult<Value> {
    if !registers.is_empty() {
        return Err(VMError::new_vm(
            "sizeof-heap-object: takes no arguments".to_string(),
        ));
    }
    Ok(Value::UInt32(SloshVm::sizeof_heap_object() as u32))
}

pub fn sizeof_value(_vm: &mut SloshVm, registers: &[Value]) -> VMResult<Value> {
    if !registers.is_empty() {
        return Err(VMError::new_vm(
            "sizeof-value: takes no arguments".to_string(),
        ));
    }
    Ok(Value::UInt32(std::mem::size_of::<Value>() as u32))
}

pub fn gensym(vm: &mut SloshVm, registers: &[Value]) -> VMResult<Value> {
    if !registers.is_empty() {
        return Err(VMError::new_vm("gensym: takes no arguments".to_string()));
    }
    let line = vm.env().line();
    let sym_idx = vm.env_mut().next_gensym();
    let sym = vm.intern(&format!("#<SYM:{line}:{sym_idx}>"));
    Ok(Value::Symbol(sym))
}
