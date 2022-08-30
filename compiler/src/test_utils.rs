use crate::{compile, CompileEnvironment, CompileState, ReadError, Reader};
use slvm::*;
use std::sync::Arc;

/// Read text for a test.  Will convert multiple forms into a vector of Values.
pub fn read_test(vm: &mut Vm, text: &'static str) -> Value {
    let reader = Reader::from_string(text.to_string(), vm, "", 1, 0);
    let exps: Vec<Value> = reader.collect::<Result<Vec<Value>, ReadError>>().unwrap();
    // Don't exit early without unpausing....
    vm.pause_gc();
    let res = if exps.len() == 1 {
        exps[0]
    } else {
        vm.alloc_vector_ro(exps)
    };
    vm.unpause_gc();
    res
}

/// Read input, compile and execute the result and return the Value this produces.
pub fn exec(vm: &mut Vm, input: &'static str) -> Value {
    let exp = read_test(vm, input);
    let mut env = CompileEnvironment::new(vm);
    let mut state = CompileState::new();
    compile(&mut env, &mut state, exp, 0).unwrap();
    state.chunk.encode0(RET, Some(1)).unwrap();
    vm.execute(Arc::new(state.chunk)).unwrap();
    vm.stack()[0]
}

/// Same as exec() but dump the registers and disassembled bytecode after executing.
/// Only use this when debugging a test, otherwise use exec().
pub fn exec_with_dump(vm: &mut Vm, input: &'static str) -> Value {
    let exp = read_test(vm, input);
    let mut env = CompileEnvironment::new(vm);
    let mut state = CompileState::new();
    compile(&mut env, &mut state, exp, 0).unwrap();
    state.chunk.encode0(RET, Some(1)).unwrap();
    env.vm_mut().execute(Arc::new(state.chunk.clone())).unwrap();

    let mut reg_names = state.chunk.dbg_args.as_ref().map(|iargs| iargs.iter());
    for (i, r) in env.vm().stack()[0..=state.chunk.extra_regs]
        .iter()
        .enumerate()
    {
        let aname = if i == 0 {
            "params/result"
        } else if let Some(reg_names) = reg_names.as_mut() {
            if let Some(n) = reg_names.next() {
                env.vm().get_interned(*n)
            } else {
                "[SCRATCH]"
            }
        } else {
            "[SCRATCH]"
        };
        if let Value::Value(_) = r {
            println!(
                "{:#03} ^{:#20}: {:#12} {}",
                i,
                aname,
                r.display_type(env.vm()),
                r.pretty_value(env.vm())
            );
        } else {
            println!(
                "{:#03}  {:#20}: {:#12} {}",
                i,
                aname,
                r.display_type(env.vm()),
                r.pretty_value(env.vm())
            );
        }
    }
    let _ = state.chunk.disassemble_chunk(env.vm(), 0);

    vm.stack()[0]
}

/// Read and compile input and fail if compiling does not result in an error.
pub fn exec_compile_error(vm: &mut Vm, input: &'static str) {
    let exp = read_test(vm, input);
    let mut env = CompileEnvironment::new(vm);
    let mut state = CompileState::new();
    assert!(
        compile(&mut env, &mut state, exp, 0).is_err(),
        "expected compile error"
    );
    vm.reset();
}

/// Read, compile and execute input and fail if execution does not result in an error.
pub fn exec_runtime_error(vm: &mut Vm, input: &'static str) {
    let exp = read_test(vm, input);
    let mut env = CompileEnvironment::new(vm);
    let mut state = CompileState::new();
    compile(&mut env, &mut state, exp, 0).unwrap();
    state.chunk.encode0(RET, Some(1)).unwrap();
    assert!(
        vm.execute(Arc::new(state.chunk)).is_err(),
        "expected runtime error"
    );
    vm.reset();
}

/// Assert that val1 and val2 are the same.
pub fn assert_vals(vm: &Vm, val1: Value, val2: Value) {
    let res = vm
        .is_equal_pair(val1, val2)
        .unwrap_or(Value::False)
        .is_true();
    if !res {
        println!(
            "Value {} != {}",
            val1.display_value(vm),
            val2.display_value(vm)
        );
        println!("Debug {:?} / {:?}", val1, val2);
    }
    assert!(res);
}
