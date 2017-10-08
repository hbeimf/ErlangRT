//! Generated by codegen/create_vm_dispatch.py
//! Dispatch for all opcode types.
//! Config used: OTP20 
#![allow(dead_code)]

use beam::gen_op::OPCODE;
use emulator::runtime_ctx::Context;
use defs::{Word, DispatchResult};
use beam::vm_opcode::*;


#[inline(always)]
pub fn dispatch_op_inline(op: OPCODE, ctx: &mut Context) -> DispatchResult {
  match op {

    OPCODE::Call => { return opcode_call(ctx) },
    OPCODE::CallLast => { return opcode_call_last(ctx) },
    OPCODE::CallOnly => { return opcode_call_only(ctx) },
    other => panic!("vm_dispatch: Opcode {:?} not implemented", other),   
  }
}

