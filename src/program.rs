use std::collections::HashMap;

use crate::{
  common::{AccessSize, BASIC_COST_ARITH_INT_MULDIV, MAIN_NAME},
  error::{SwppError, SwppRawResult, SwppResult},
  function::SwppFunction,
  inst::{InstStdRead, InstStdWrite, SwppInst, SwppInstKind},
  logger::SwppLogger,
  memory::SwppMemory,
  register::{SwppRegisterName, SwppRegisterSet},
};

/// Result wrapper for block execution
pub enum BlockResult {
  /// Fetch a name of the block to jump
  /// and the line number of the last instruction
  NextBlock(String, u64),
  Return(Option<u64>),
  Call(CallRequest),
}

#[derive(Debug, Clone)]
pub struct CallRequest {
  pub fname: String,
  pub args: Vec<u64>,
  pub target: Option<SwppRegisterName>,
  pub loc: u64,
}

/// A block of instructions, which is a part of function implementation
/// It's composed of a name, a vector of instructions
/// and the start location for error reporting
#[derive(Debug, Clone)]
pub struct SwppBlock {
  pub block_name: String,
  pub inst_vec: Vec<SwppInst>,
  pub start_loc: u64,
}

/// **Global State**
/// It indicates the current execution context,
/// which describes which function and block is currently being executed.
/// It also has the memory, cost paid so far and the function implementations in the program
pub struct SwppState {
  mem: SwppMemory,
  cur_cost: u64,
  /// HashMap from function name to function implementation,
  /// which is used to access function implementation by function name when calling a function
  functions: HashMap<String, SwppFunction>,
  /// Current execution context, which contains the name of function and block currently being executed.
  cur_context: SwppContext,
  /// Outstanding async load debts: register -> debt amount.
  aload_debts: HashMap<SwppRegisterName, u64>,
  /// Deferred mul/div/rem cost that may be canceled by the immediately following add/sub.
  pending_fma_cost: Option<PendingFmaCost>,
}

impl SwppState {
  pub fn new(mut f_vec: Vec<SwppFunction>) -> Self {
    let read_block = SwppBlock {
      block_name: "read".to_owned(),
      inst_vec: vec![SwppInst::new(SwppInstKind::Read(InstStdRead::new()), 0)],
      start_loc: 0,
    };
    let read_fun = SwppFunction::new("read".to_owned(), 0, vec![read_block]);

    let write_block = SwppBlock {
      block_name: "write".to_owned(),
      inst_vec: vec![SwppInst::new(SwppInstKind::Write(InstStdWrite::new()), 0)],
      start_loc: 0,
    };
    let write_fun = SwppFunction::new("write".to_owned(), 1, vec![write_block]);

    f_vec.push(read_fun);
    f_vec.push(write_fun);

    let functions: HashMap<String, SwppFunction> =
      f_vec.into_iter().map(|f| (f.fname(), f)).collect();

    Self {
      mem: SwppMemory::new(),
      cur_cost: 0,
      functions,
      cur_context: SwppContext {
        fname: "main".to_owned(),
        bname: "entry".to_owned(),
      },
      aload_debts: HashMap::new(),
      pending_fma_cost: None,
    }
  }

  pub fn get_fn_ref(&self, fname: &str) -> SwppRawResult<&SwppFunction> {
    self
      .functions
      .get(fname)
      .ok_or(crate::error::SwppErrorKind::UnknownFnName(fname.to_owned()))
  }

  pub fn get_block_index(&self, fname: &str, block_name: &str) -> SwppRawResult<u64> {
    self
      .functions
      .get(fname)
      .ok_or(crate::error::SwppErrorKind::UnknownFnName(fname.to_owned()))?
      .get_block_index(block_name)
  }
  pub fn add_cost(&mut self, cost: u64) {
    self.cur_cost += cost;
  }
  pub fn get_cost(&self) -> u64 {
    self.cur_cost
  }
  pub fn change_f_context(&mut self, fname: &str) {
    self.cur_context.fname = fname.to_owned();
  }
  pub fn change_b_context(&mut self, bname: &str) {
    self.cur_context.bname = bname.to_owned();
  }
  pub fn get_context(&self) -> &SwppContext {
    &self.cur_context
  }
  pub fn has_pending_fma_cost(&self) -> bool {
    self.pending_fma_cost.is_some()
  }
  pub fn defer_fma_cost(&mut self, loc: u64, inst_name: &str) {
    self.pending_fma_cost = Some(PendingFmaCost {
      loc,
      inst_name: inst_name.to_owned(),
      context: self.cur_context.clone(),
    });
  }
  pub fn flush_pending_fma_cost<L: SwppLogger>(&mut self, logger: &mut L) {
    let Some(pending_inst) = self.pending_fma_cost.take() else {
      return;
    };

    self.add_cost(BASIC_COST_ARITH_INT_MULDIV);
    self.cool_down(BASIC_COST_ARITH_INT_MULDIV, None);
    self.resolve_aload_debts(BASIC_COST_ARITH_INT_MULDIV, None);
    logger.log(
      pending_inst.loc,
      &pending_inst.inst_name,
      BASIC_COST_ARITH_INT_MULDIV,
      self.get_cost(),
      &pending_inst.context,
    );
  }
  pub fn cancel_pending_fma_cost<L: SwppLogger>(&mut self, logger: &mut L) {
    let Some(pending_inst) = self.pending_fma_cost.take() else {
      return;
    };

    let log_name = format!("{}-FMA", pending_inst.inst_name);
    logger.log(
      pending_inst.loc,
      &log_name,
      0,
      self.get_cost(),
      &pending_inst.context,
    );
  }

  pub fn malloc(&mut self, size: u64) -> SwppRawResult<u64> {
    self.mem.malloc(size)
  }

  pub fn free(&mut self, addr: u64) -> SwppRawResult<()> {
    self.mem.free(addr)
  }

  pub fn read_from_stack(&self, addr: u64, size: AccessSize) -> SwppRawResult<u64> {
    self.mem.read_from_stack(addr, size)
  }

  pub fn read_from_heap(&self, addr: u64, size: AccessSize) -> SwppRawResult<u64> {
    self.mem.read_from_heap(addr, size)
  }

  pub fn write_to_stack(&mut self, addr: u64, val: u64, size: AccessSize) -> SwppRawResult<()> {
    self.mem.write_to_stack(addr, val, size)
  }
  pub fn write_to_heap(&mut self, addr: u64, val: u64, size: AccessSize) -> SwppRawResult<()> {
    self.mem.write_to_heap(addr, val, size)
  }

  pub fn cost_stack(&mut self, addr: u64, size: AccessSize) -> (u64, Vec<u64>) {
    self.mem.cost_stack(addr, size)
  }

  pub fn cost_heap(&mut self, addr: u64, size: AccessSize) -> (u64, Vec<u64>) {
    self.mem.cost_heap(addr, size)
  }

  pub fn cool_down(&mut self, instruction_cost: u64, exclude_sectors: Option<&[u64]>) {
    self.mem.cool_down(instruction_cost, exclude_sectors);
  }

  /// Create an async load debt for a register
  pub fn create_aload_debt(&mut self, reg: SwppRegisterName, debt: u64) {
    self.aload_debts.insert(reg, debt);
  }

  /// Get the debt amount for a register (0 if no debt)
  pub fn get_aload_debt(&self, reg: &SwppRegisterName) -> u64 {
    self.aload_debts.get(reg).copied().unwrap_or(0)
  }

  /// Resolve debt using instruction cost (returns amount actually resolved)
  pub fn resolve_aload_debt(&mut self, reg: &SwppRegisterName, available_cost: u64) -> u64 {
    if let Some(debt) = self.aload_debts.get_mut(reg) {
      let resolved = available_cost.min(*debt);
      *debt -= resolved;
      if *debt == 0 {
        self.aload_debts.remove(reg);
      }
      resolved
    } else {
      0
    }
  }

  /// Pay back outstanding debt for a register (when it's used before resolution)
  pub fn pay_aload_debt(&mut self, reg: &SwppRegisterName) -> u64 {
    self.aload_debts.remove(reg).unwrap_or(0)
  }

  /// Cancel debt for a register (when it's overwritten)
  pub fn cancel_aload_debt(&mut self, reg: &SwppRegisterName) {
    self.aload_debts.remove(reg);
  }

  /// Resolve async load debts using elapsed instruction cost, excluding the
  /// freshly-created async-load result when needed.
  pub fn resolve_aload_debts(
    &mut self,
    instruction_cost: u64,
    exclude_reg: Option<&SwppRegisterName>,
  ) {
    if instruction_cost == 0 {
      return;
    }

    self.aload_debts.retain(|reg, debt| {
      if exclude_reg.map_or(false, |exclude| reg == exclude) {
        true
      } else {
        *debt = debt.saturating_sub(instruction_cost);
        *debt > 0
      }
    });
  }

  /// Read from register, paying back any outstanding debt
  pub fn read_register_with_debt(
    &mut self,
    reg_set: &SwppRegisterSet,
    reg: &SwppRegisterName,
  ) -> SwppRawResult<u64> {
    let debt = self.pay_aload_debt(reg);
    if debt > 0 {
      self.add_cost(debt);
      self.cool_down(debt, None);
      self.resolve_aload_debts(debt, None);
    }
    reg_set.read_register_word(reg)
  }

  /// Write to register, canceling any outstanding debt
  pub fn write_register_with_debt(
    &mut self,
    reg_set: &mut SwppRegisterSet,
    reg: &SwppRegisterName,
    val: u64,
  ) -> SwppRawResult<()> {
    if reg.is_arg() {
      return Err(crate::error::SwppErrorKind::ArgRegAssign(reg.clone()));
    }
    self.cancel_aload_debt(reg);
    reg_set.write_register_word(reg, val)
  }
}

#[derive(Debug, Clone)]
pub struct SwppContext {
  /// A name of the function currently being executed
  fname: String,
  /// A name of the block currently being executed
  bname: String,
}

#[derive(Debug, Clone)]
struct PendingFmaCost {
  loc: u64,
  inst_name: String,
  context: SwppContext,
}

impl SwppContext {
  pub fn get_fname(&self) -> String {
    self.fname.to_owned()
  }
  pub fn get_bname(&self) -> String {
    self.bname.to_owned()
  }
}

/// Whole program parsed from given IR
pub struct SwppProgram<L> {
  /// Global State of current Program
  state: SwppState,
  logger: L,
}

#[derive(Clone)]
struct CallSite {
  loc: u64,
  target: Option<SwppRegisterName>,
}

struct CallFrame {
  fname: String,
  reg_set: SwppRegisterSet,
  current_block: String,
  next_inst_idx: usize,
  call_site: Option<CallSite>,
}

impl<'a, L: SwppLogger> SwppProgram<L> {
  pub fn new(state: SwppState, logger: L) -> Self {
    Self { state, logger }
  }

  pub fn total_cost(&self) -> u64 {
    self.state.cur_cost + self.state.mem.get_max_heap_size() * 1024
  }

  fn create_call_frame(
    &self,
    fname: &str,
    prev_reg: &SwppRegisterSet,
    args: Vec<u64>,
    call_site: Option<CallSite>,
  ) -> SwppRawResult<CallFrame> {
    let function = self.state.get_fn_ref(fname)?;
    let mut reg_set = prev_reg.clone();
    reg_set.set_arg_register(&args);

    Ok(CallFrame {
      fname: fname.to_owned(),
      reg_set,
      current_block: function.get_entry_block_name(),
      next_inst_idx: 0,
      call_site,
    })
  }

  fn wrap_call_stack_error(&self, mut err: SwppError, call_stack: &[CallFrame]) -> SwppError {
    for frame in call_stack.iter().rev() {
      if let Some(call_site) = &frame.call_site {
        err = SwppError::new(
          crate::error::SwppErrorKind::FunctionCallCrash(frame.fname.clone(), err.to_string()),
          call_site.loc,
        );
      } else {
        break;
      }
    }

    err
  }

  pub fn run(&mut self) -> SwppResult<()> {
    let main_frame = self
      .create_call_frame(MAIN_NAME, &SwppRegisterSet::default(), Vec::new(), None)
      .map_err(|err| SwppError::new(err, 0))?;

    let mut call_stack = vec![main_frame];
    self.logger.enter_fn();

    while !call_stack.is_empty() {
      let top_idx = call_stack.len() - 1;

      {
        let frame = &call_stack[top_idx];
        self.state.change_f_context(&frame.fname);
        self.state.change_b_context(&frame.current_block);
      }

      let inst = {
        let frame = &call_stack[top_idx];
        let function = self
          .state
          .get_fn_ref(&frame.fname)
          .map_err(|err| self.wrap_call_stack_error(SwppError::new(err, 0), &call_stack))?;
        let block = function
          .get_block(&frame.current_block)
          .map_err(|err| self.wrap_call_stack_error(SwppError::new(err, 0), &call_stack))?;

        block
          .inst_vec
          .get(frame.next_inst_idx)
          .cloned()
          .ok_or_else(|| {
            self.wrap_call_stack_error(
              SwppError::new(
                crate::error::SwppErrorKind::IllFormedBlock(frame.current_block.clone()),
                block.start_loc,
              ),
              &call_stack,
            )
          })?
      };

      call_stack[top_idx].next_inst_idx += 1;

      let result = inst.run(
        &mut self.state,
        &mut call_stack[top_idx].reg_set,
        &mut self.logger,
      );
      let result = match result {
        Ok(result) => result,
        Err(err) => return Err(self.wrap_call_stack_error(err, &call_stack)),
      };

      match result {
        None => {}
        Some(BlockResult::NextBlock(block_name, line_num)) => {
          let fname = call_stack[top_idx].fname.clone();
          self
            .state
            .get_fn_ref(&fname)
            .and_then(|function| function.get_block(&block_name).map(|_| ()))
            .map_err(|err| {
              self.wrap_call_stack_error(SwppError::new(err, line_num), &call_stack)
            })?;

          call_stack[top_idx].current_block = block_name;
          call_stack[top_idx].next_inst_idx = 0;
        }
        Some(BlockResult::Return(ret_val)) => {
          self.logger.exit_fn();
          let finished_frame = call_stack.pop().expect("call stack should not be empty");

          if let Some(call_site) = finished_frame.call_site {
            let caller = call_stack
              .last_mut()
              .expect("callee frame should always have a caller");

            match (call_site.target, ret_val) {
              (Some(target), Some(val)) => {
                if let Err(err) =
                  self
                    .state
                    .write_register_with_debt(&mut caller.reg_set, &target, val)
                {
                  return Err(
                    self.wrap_call_stack_error(SwppError::new(err, call_site.loc), &call_stack),
                  );
                }
              }
              (Some(_), None) => {
                return Err(self.wrap_call_stack_error(
                  SwppError::new(
                    crate::error::SwppErrorKind::AssignNoValue(finished_frame.fname),
                    call_site.loc,
                  ),
                  &call_stack,
                ))
              }
              (None, _) => {}
            }
          }
        }
        Some(BlockResult::Call(call_request)) => {
          let caller_reg = call_stack[top_idx].reg_set.clone();
          let call_site = CallSite {
            loc: call_request.loc,
            target: call_request.target,
          };
          let callee_frame = self
            .create_call_frame(
              &call_request.fname,
              &caller_reg,
              call_request.args,
              Some(call_site),
            )
            .map_err(|err| SwppError::new(err, call_request.loc))?;
          self.logger.enter_fn();
          call_stack.push(callee_frame);
        }
      }
    }

    Ok(())
  }
}
