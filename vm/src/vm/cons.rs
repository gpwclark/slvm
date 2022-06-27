use crate::{decode2, decode3, decode_u16, get_reg_unref, VMError, VMResult, Value, Vm};

impl Vm {
    pub(super) fn list(
        &mut self,
        code: &[u8],
        registers: &mut [Value],
        wide: bool,
    ) -> VMResult<()> {
        let (dest, start, end) = decode3!(code, &mut self.ip, wide);
        if end < start {
            self.set_register(registers, dest as usize, Value::Nil);
        } else {
            let mut last_cdr = Value::Nil;
            for i in (start..=end).rev() {
                let car = get_reg_unref!(registers, i, self);
                let cdr = last_cdr;
                last_cdr = self.alloc_pair(car, cdr);
            }
            self.set_register(registers, dest as usize, last_cdr);
        }
        Ok(())
    }

    pub(super) fn append(
        &mut self,
        code: &[u8],
        registers: &mut [Value],
        wide: bool,
    ) -> VMResult<()> {
        let (dest, start, end) = decode3!(code, &mut self.ip, wide);
        if end < start {
            self.set_register(registers, dest as usize, Value::Nil);
        } else {
            let mut last_cdr = Value::Nil;
            let mut head = Value::Nil;
            let mut loop_cdr;
            for i in start..=end {
                let lst = get_reg_unref!(registers, i, self);
                match lst {
                    Value::Nil => {}
                    Value::Pair(_) | Value::List(_, _) => {
                        let (car, cdr) = lst.get_pair(self).expect("Pair/List not a Pair or List?");
                        loop_cdr = cdr;
                        let cdr = last_cdr;
                        last_cdr = self.alloc_pair(car, Value::Nil);
                        match cdr {
                            Value::Nil => head = last_cdr,
                            Value::Pair(h) => {
                                let (_, cdr) = self.get_pair_mut(h)?;
                                *cdr = last_cdr;
                            }
                            Value::List(_, _) => {
                                return Err(VMError::new_heap("Pair is not mutable!"))
                            }
                            _ => {}
                        }
                        loop {
                            if let Value::Nil = loop_cdr {
                                break;
                            }
                            match loop_cdr {
                                Value::Pair(_) | Value::List(_, _) => {
                                    let (car, ncdr) = loop_cdr
                                        .get_pair(self)
                                        .expect("Pair/List not a Pair or List?");
                                    loop_cdr = ncdr;
                                    let cdr = last_cdr;
                                    last_cdr = self.alloc_pair(car, Value::Nil);
                                    match cdr {
                                        Value::Nil => head = last_cdr,
                                        Value::Pair(h) => {
                                            let (_, cdr) = self.get_pair_mut(h)?;
                                            *cdr = last_cdr;
                                        }
                                        Value::List(_, _) => {
                                            return Err(VMError::new_heap("Pair is not mutable!"))
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {
                                    if i == end {
                                        match last_cdr {
                                            Value::Nil => head = loop_cdr,
                                            Value::Pair(h) => {
                                                let (_, cdr) = self.get_pair_mut(h)?;
                                                *cdr = loop_cdr;
                                            }
                                            Value::List(_, _) => {
                                                return Err(VMError::new_heap(
                                                    "Pair is not mutable!",
                                                ))
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        return Err(VMError::new_vm("APND: Param not a list."));
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    _ => {
                        if i == end {
                            match last_cdr {
                                Value::Nil => head = lst,
                                Value::Pair(h) => {
                                    let (_, cdr) = self.get_pair_mut(h)?;
                                    *cdr = lst;
                                }
                                Value::List(_, _) => {
                                    return Err(VMError::new_heap("Pair is not mutable!"))
                                }
                                _ => {}
                            }
                        } else {
                            return Err(VMError::new_vm("APND: Param not a list."));
                        }
                    }
                }
            }
            self.set_register(registers, dest as usize, head);
        }
        Ok(())
    }

    pub(super) fn xar(&mut self, code: &[u8], registers: &mut [Value], wide: bool) -> VMResult<()> {
        let (pair_reg, val) = decode2!(code, &mut self.ip, wide);
        let pair = get_reg_unref!(registers, pair_reg, self);
        let val = get_reg_unref!(registers, val, self);
        match &pair {
            Value::Pair(handle) => {
                let (car, _) = self.get_pair_mut(*handle)?;
                *car = val;
            }
            Value::Nil => {
                let pair = self.alloc_pair(val, Value::Nil);
                self.set_register(registers, pair_reg as usize, pair);
            }
            Value::List(_, _) => return Err(VMError::new_vm("XAR: Pair is read only.")),
            _ => return Err(VMError::new_vm("XAR: Not a pair/conscell.")),
        }
        Ok(())
    }

    pub(super) fn xdr(&mut self, code: &[u8], registers: &mut [Value], wide: bool) -> VMResult<()> {
        let (pair_reg, val) = decode2!(code, &mut self.ip, wide);
        let pair = get_reg_unref!(registers, pair_reg, self);
        let val = get_reg_unref!(registers, val, self);
        match &pair {
            Value::Pair(handle) => {
                let (_, cdr) = self.get_pair_mut(*handle)?;
                *cdr = val;
            }
            Value::Nil => {
                let pair = self.alloc_pair(Value::Nil, val);
                self.set_register(registers, pair_reg as usize, pair);
            }
            Value::List(_, _) => return Err(VMError::new_vm("XDR: Pair is read only.")),
            _ => return Err(VMError::new_vm("XDR: Not a pair/conscell.")),
        }
        Ok(())
    }
}
