use std::collections::HashMap;

use derive_name::VariantName;
use derive_new::new;
use text_io::read;

use crate::{
  common::{
    AccessSize, Arg, BitWidth, BASIC_COST_ARITH_BITWISE, BASIC_COST_ARITH_CMP,
    BASIC_COST_ARITH_INT_ADDSUB_RIGHT, BASIC_COST_ARITH_INT_ADDSUB_WRONG,
    BASIC_COST_ARITH_INT_SHIFT, BASIC_COST_ARITH_SELECT, BASIC_COST_CONTROL_BRANCH_FALSE,
    BASIC_COST_CONTROL_BRANCH_TRUE, BASIC_COST_CONTROL_FNCALL, BASIC_COST_CONTROL_RET,
    BASIC_COST_CONTROL_SWITCH, BASIC_COST_CONTROL_UBRANCH, BASIC_COST_MEM_HEAP_OP,
    FORWARD_BRANCH_COST_MULTIPLIER, HEAP_OFFSET, ICMP, MEMORY_ACCESS_COST_HEAP,
    MEMORY_ACCESS_COST_STACK, MEM_STACK_SIZE,
  },
  error::{SwppError, SwppErrorKind, SwppRawResult, SwppResult},
  logger::SwppLogger,
  program::{BlockResult, CallRequest, SwppState},
  register::{SwppRegisterName, SwppRegisterSet},
};

fn scaled_forward_branch_cost(base_cost: u64) -> u64 {
  ((base_cost as f64) * FORWARD_BRANCH_COST_MULTIPLIER) as u64
}

fn is_forward_jump(state: &mut SwppState, target_block: &str) -> SwppRawResult<bool> {
  let fname = state.get_context().get_fname();
  let current_block = state.get_context().get_bname();
  let current_block_num = state.get_block_index(&fname, &current_block)?;
  let target_block_num = state.get_block_index(&fname, target_block)?;
  Ok(target_block_num > current_block_num)
}

/// General form of instruction
#[derive(Debug, Clone, new)]
pub struct SwppInst {
  kind: SwppInstKind,
  loc: u64,
}

impl SwppInst {
  pub fn get_kind(&self) -> SwppInstKind {
    self.kind.clone()
  }

  fn finalize_instruction<L: SwppLogger>(
    &self,
    state: &mut SwppState,
    logger: &mut L,
    inst_name: &str,
    log_name: &str,
    cost: u64,
    four_phobia_args: &[&Arg],
    four_phobia_size: Option<AccessSize>,
    exclude_sectors: Option<&[u64]>,
  ) {
    self.finalize_instruction_inner(
      state,
      logger,
      inst_name,
      log_name,
      cost,
      four_phobia_args,
      four_phobia_size,
      None,
      exclude_sectors,
      true,
    );
  }

  fn finalize_instruction_without_base_log<L: SwppLogger>(
    &self,
    state: &mut SwppState,
    logger: &mut L,
    inst_name: &str,
    cost: u64,
    four_phobia_args: &[&Arg],
    four_phobia_size: Option<AccessSize>,
    exclude_sectors: Option<&[u64]>,
  ) {
    self.finalize_instruction_inner(
      state,
      logger,
      inst_name,
      inst_name,
      cost,
      four_phobia_args,
      four_phobia_size,
      None,
      exclude_sectors,
      false,
    );
  }

  fn finalize_instruction_preserving_aload_debt<L: SwppLogger>(
    &self,
    state: &mut SwppState,
    logger: &mut L,
    inst_name: &str,
    log_name: &str,
    cost: u64,
    four_phobia_args: &[&Arg],
    four_phobia_size: Option<AccessSize>,
    preserved_reg: &SwppRegisterName,
    exclude_sectors: Option<&[u64]>,
  ) {
    self.finalize_instruction_inner(
      state,
      logger,
      inst_name,
      log_name,
      cost,
      four_phobia_args,
      four_phobia_size,
      Some(preserved_reg),
      exclude_sectors,
      true,
    );
  }

  fn finalize_instruction_inner<L: SwppLogger>(
    &self,
    state: &mut SwppState,
    logger: &mut L,
    inst_name: &str,
    log_name: &str,
    cost: u64,
    four_phobia_args: &[&Arg],
    four_phobia_size: Option<AccessSize>,
    exclude_reg: Option<&SwppRegisterName>,
    exclude_sectors: Option<&[u64]>,
    log_base_cost: bool,
  ) {
    let cost_before = state.get_cost();
    for arg in four_phobia_args {
      arg.add_cost_if_four_phobia(state, logger, self.loc, inst_name);
      if state.get_cost() > cost_before {
        break;
      }
    }
    if state.get_cost() == cost_before {
      if let Some(size) = four_phobia_size {
        size.add_cost_if_four_phobia(state, logger, self.loc, inst_name);
      }
    }
    let extra_cost = state.get_cost() - cost_before;
    let elapsed_cost = cost + extra_cost;
    state.add_cost(cost);
    state.cool_down(elapsed_cost, exclude_sectors);
    state.resolve_aload_debts(elapsed_cost, exclude_reg);
    if log_base_cost {
      logger.log(
        self.loc,
        log_name,
        cost,
        state.get_cost(),
        state.get_context(),
      );
    }
  }

  pub fn run<L: SwppLogger>(
    &self,
    state: &mut SwppState,
    reg_set: &mut SwppRegisterSet,
    logger: &mut L,
  ) -> SwppResult<Option<BlockResult>> {
    let inst_name = self.kind.variant_name();
    let is_fma_addsub = matches!(
      &self.kind,
      SwppInstKind::EAdd(_) | SwppInstKind::OAdd(_) | SwppInstKind::ESub(_) | SwppInstKind::OSub(_)
    ) && state.has_pending_fma_cost();
    if !is_fma_addsub {
      state.flush_pending_fma_cost(logger);
    }
    let ret = match &self.kind {
      SwppInstKind::Ret(ret) => {
        let cost = BASIC_COST_CONTROL_RET;
        let ret_val = ret
          .val
          .clone()
          .map(|arg| arg.read_value_with_state(state, reg_set))
          .transpose()
          .map_err(|err| SwppError::new(err, self.loc))?;
        let four_phobia_args: Vec<&Arg> = ret.val.iter().collect();
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &four_phobia_args,
          None,
          None,
        );
        Ok(Some(BlockResult::Return(ret_val)))
      }
      SwppInstKind::UBranch(br) => {
        let cost =
          if is_forward_jump(state, &br.target).map_err(|err| SwppError::new(err, self.loc))? {
            scaled_forward_branch_cost(BASIC_COST_CONTROL_UBRANCH)
          } else {
            BASIC_COST_CONTROL_UBRANCH
          };
        self.finalize_instruction(state, logger, &inst_name, &inst_name, cost, &[], None, None);
        Ok(Some(BlockResult::NextBlock(br.target.clone(), self.loc)))
      }
      SwppInstKind::Branch(br) => {
        let (next_block, cost) = br
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&br.cond_reg],
          None,
          None,
        );
        Ok(Some(BlockResult::NextBlock(next_block, self.loc)))
      }
      SwppInstKind::Switch(sw) => {
        let next_block = sw
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        let cost = BASIC_COST_CONTROL_SWITCH;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&sw.cond_reg],
          None,
          None,
        );
        Ok(Some(BlockResult::NextBlock(next_block, self.loc)))
      }
      SwppInstKind::FnCall(fcall) => {
        let cost = BASIC_COST_CONTROL_FNCALL;
        let arg_set = fcall
          .evaluate_args(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        let four_phobia_args: Vec<&Arg> = fcall.args.iter().collect();
        let log_name = format!("{}-{}", inst_name, fcall.fname);
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &log_name,
          cost,
          &four_phobia_args,
          None,
          None,
        );
        Ok(Some(BlockResult::Call(CallRequest {
          fname: fcall.fname.clone(),
          args: arg_set,
          target: fcall.target.clone(),
          loc: self.loc,
        })))
      }
      SwppInstKind::Assert(assertion) => {
        assertion
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        // `assert_eq` is a testing-only helper, so it never invokes 4-phobia.
        self.finalize_instruction(state, logger, &inst_name, &inst_name, 0, &[], None, None);
        Ok(None)
      }
      SwppInstKind::Malloc(malloc) => {
        let cost = BASIC_COST_MEM_HEAP_OP;
        malloc
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&malloc.size_reg],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Free(free) => {
        let cost = BASIC_COST_MEM_HEAP_OP;
        free
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&free.addr_reg],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Load(load) => {
        let (cost, affected_sectors) = load
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&load.addr_reg],
          Some(load.size),
          Some(&affected_sectors),
        );
        Ok(None)
      }
      SwppInstKind::Store(store) => {
        let (cost, affected_sectors) = store
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&store.val_reg, &store.addr_reg],
          Some(store.size),
          Some(&affected_sectors),
        );
        Ok(None)
      }
      SwppInstKind::ALoad(aload) => {
        let (cost, affected_sectors) = aload
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction_preserving_aload_debt(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&aload.addr_reg],
          Some(aload.size),
          &aload.target_reg,
          Some(&affected_sectors),
        );
        Ok(None)
      }
      SwppInstKind::UDiv(udiv) => {
        udiv
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction_without_base_log(
          state,
          logger,
          &inst_name,
          0,
          &[&udiv.reg1, &udiv.reg2],
          None,
          None,
        );
        state.defer_fma_cost(self.loc, &inst_name);
        Ok(None)
      }
      SwppInstKind::SDiv(sdiv) => {
        sdiv
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction_without_base_log(
          state,
          logger,
          &inst_name,
          0,
          &[&sdiv.reg1, &sdiv.reg2],
          None,
          None,
        );
        state.defer_fma_cost(self.loc, &inst_name);
        Ok(None)
      }
      SwppInstKind::URem(urem) => {
        urem
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction_without_base_log(
          state,
          logger,
          &inst_name,
          0,
          &[&urem.reg1, &urem.reg2],
          None,
          None,
        );
        state.defer_fma_cost(self.loc, &inst_name);
        Ok(None)
      }
      SwppInstKind::SRem(srem) => {
        srem
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction_without_base_log(
          state,
          logger,
          &inst_name,
          0,
          &[&srem.reg1, &srem.reg2],
          None,
          None,
        );
        state.defer_fma_cost(self.loc, &inst_name);
        Ok(None)
      }
      SwppInstKind::Mul(mul) => {
        mul
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction_without_base_log(
          state,
          logger,
          &inst_name,
          0,
          &[&mul.reg1, &mul.reg2],
          None,
          None,
        );
        state.defer_fma_cost(self.loc, &inst_name);
        Ok(None)
      }
      SwppInstKind::Shl(shl) => {
        let cost = BASIC_COST_ARITH_INT_SHIFT;
        shl
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&shl.reg1, &shl.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Lshr(lshr) => {
        let cost = BASIC_COST_ARITH_INT_SHIFT;
        lshr
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&lshr.reg1, &lshr.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Ashr(ashr) => {
        let cost = BASIC_COST_ARITH_INT_SHIFT;
        ashr
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&ashr.reg1, &ashr.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::And(and) => {
        let cost = BASIC_COST_ARITH_BITWISE;
        and
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&and.reg1, &and.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Or(or) => {
        let cost = BASIC_COST_ARITH_BITWISE;
        or.run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&or.reg1, &or.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Xor(xor) => {
        let cost = BASIC_COST_ARITH_BITWISE;
        xor
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&xor.reg1, &xor.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::EAdd(eadd) => {
        let cost = eadd
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        if is_fma_addsub {
          state.cancel_pending_fma_cost(logger);
        }
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&eadd.reg1, &eadd.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::OAdd(oadd) => {
        let cost = oadd
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        if is_fma_addsub {
          state.cancel_pending_fma_cost(logger);
        }
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&oadd.reg1, &oadd.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::ESub(esub) => {
        let cost = esub
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        if is_fma_addsub {
          state.cancel_pending_fma_cost(logger);
        }
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&esub.reg1, &esub.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::OSub(osub) => {
        let cost = osub
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        if is_fma_addsub {
          state.cancel_pending_fma_cost(logger);
        }
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&osub.reg1, &osub.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Comp(comp) => {
        let cost = BASIC_COST_ARITH_CMP;
        comp
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&comp.reg1, &comp.reg2],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Select(select) => {
        let cost = BASIC_COST_ARITH_SELECT;
        select
          .run(state, reg_set)
          .map_err(|err| SwppError::new(err, self.loc))?;
        self.finalize_instruction(
          state,
          logger,
          &inst_name,
          &inst_name,
          cost,
          &[&select.cond_reg, &select.true_reg, &select.false_reg],
          None,
          None,
        );
        Ok(None)
      }
      SwppInstKind::Read(read) => read
        .run()
        .map(|v| Some(BlockResult::Return(Some(v))))
        .map_err(|err| SwppError::new(err, self.loc)),
      SwppInstKind::Write(write) => write
        .run(reg_set)
        .map(|_| Some(BlockResult::Return(None)))
        .map_err(|err| SwppError::new(err, self.loc)),
    };

    ret
  }
}

#[derive(Debug, Clone, VariantName)]
pub enum SwppInstKind {
  Ret(InstRet),
  UBranch(InstUncondBr),
  Branch(InstCondBr),
  Switch(InstSwitch),
  FnCall(InstFunctionCall),
  Assert(InstAssertion),
  Malloc(InstHeapAllocation),
  Free(InstHeapFree),
  Load(InstLoad),
  Store(InstStore),
  ALoad(InstAsyncLoad),
  UDiv(InstUnsignedDivision),
  SDiv(InstSignedDivision),
  URem(InstUnsignedRemainder),
  SRem(InstSignedRemainder),
  Mul(InstMultiplication),
  Shl(InstShiftLeft),
  Lshr(InstShiftRightLogical),
  Ashr(InstShiftRightArithmetic),
  And(InstBitwiseAnd),
  Or(InstBitwiseOr),
  Xor(InstBitwiseXor),
  EAdd(InstEAdd),
  OAdd(InstOAdd),
  ESub(InstESub),
  OSub(InstOSub),
  Comp(InstComparison),
  Select(InstTernary),
  Read(InstStdRead),
  Write(InstStdWrite),
}

/// Return Value
#[derive(Debug, Clone, new)]
pub struct InstRet {
  val: Option<Arg>,
}

/// Unconditional Branch
#[derive(Debug, Clone, new)]
pub struct InstUncondBr {
  target: String,
}

#[derive(Debug, Clone, new)]
/// Conditional Branch
pub struct InstCondBr {
  cond_reg: Arg,
  true_target: String,
  false_target: String,
}

impl InstCondBr {
  /// Return a name of the next block and the cost according to the value of condition register
  pub fn run(
    &self,
    state: &mut SwppState,
    reg_set: &mut SwppRegisterSet,
  ) -> SwppRawResult<(String, u64)> {
    let cond = self.cond_reg.read_value_with_state(state, reg_set)?;
    let next_block = if cond == 0 {
      &self.false_target
    } else {
      &self.true_target
    };

    let mut cost = match cond {
      0 => BASIC_COST_CONTROL_BRANCH_FALSE,
      1 => BASIC_COST_CONTROL_BRANCH_TRUE,
      _ => return Err(SwppErrorKind::InvalidCondVal(cond)),
    };

    if is_forward_jump(state, next_block)? {
      cost = scaled_forward_branch_cost(cost);
    }

    match cond {
      0 => Ok((self.false_target.clone(), cost)),
      1 => Ok((self.true_target.clone(), cost)),
      _ => Err(SwppErrorKind::InvalidCondVal(cond)),
    }
  }
}

#[derive(Debug, Clone, new)]
/// Switch
pub struct InstSwitch {
  cond_reg: Arg,
  /// Mapping from the value of condition register to the name of next block
  jump_map: HashMap<u64, String>,
  default_block: String,
}

impl InstSwitch {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<String> {
    let cond = self.cond_reg.read_value_with_state(state, reg_set)?;
    let next_block = self
      .jump_map
      .get(&cond)
      .unwrap_or(&self.default_block)
      .clone();
    Ok(next_block)
  }
}

#[derive(Debug, Clone, new)]
/// Function call
pub struct InstFunctionCall {
  target: Option<SwppRegisterName>,
  fname: String,
  args: Vec<Arg>,
}

impl InstFunctionCall {
  pub fn evaluate_args(
    &self,
    state: &mut SwppState,
    reg_set: &mut SwppRegisterSet,
  ) -> SwppRawResult<Vec<u64>> {
    let function = state.get_fn_ref(&self.fname)?;

    // Check the number of arguments
    if self.args.len() as u64 != function.nargs() {
      return Err(SwppErrorKind::WrongArgNum(
        self.fname.clone(),
        function.nargs(),
        self.args.len() as u64,
      ));
    }

    self
      .args
      .iter()
      .map(|x| x.read_value_with_state(state, reg_set))
      .collect()
  }
}

#[derive(Debug, Clone, new)]
pub struct InstAssertion {
  lhs: Arg,
  rhs: Arg,
}

impl InstAssertion {
  pub fn run(&self, state: &mut SwppState, reg_set: &SwppRegisterSet) -> SwppRawResult<()> {
    let lhs_val = self.lhs.read_value_with_state(state, reg_set)?;
    let rhs_val = self.rhs.read_value_with_state(state, reg_set)?;
    if rhs_val == lhs_val {
      Ok(())
    } else {
      Err(SwppErrorKind::AssertionFailed(rhs_val, lhs_val))
    }
  }
}

/// Malloc
#[derive(Debug, Clone, new)]
pub struct InstHeapAllocation {
  target_reg: SwppRegisterName,
  size_reg: Arg,
}

impl InstHeapAllocation {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let size_u64 = self.size_reg.read_value_with_state(state, reg_set)?;
    // Check size
    if size_u64 == 0 || size_u64 % 8 != 0 {
      return Err(SwppErrorKind::InvalidHeapAllocSize(size_u64));
    }

    let addr = state.malloc(size_u64)?;

    state.write_register_with_debt(reg_set, &self.target_reg, addr)
  }
}

/// Free
#[derive(Debug, Clone, new)]
pub struct InstHeapFree {
  addr_reg: Arg,
}

impl InstHeapFree {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let addr = self.addr_reg.read_value_with_state(state, reg_set)?;

    state.free(addr)
  }
}

/// Load
#[derive(Debug, Clone, new)]
pub struct InstLoad {
  target_reg: SwppRegisterName,
  addr_reg: Arg,
  size: AccessSize,
}

impl InstLoad {
  pub fn run(
    &self,
    state: &mut SwppState,
    reg_set: &mut SwppRegisterSet,
  ) -> SwppRawResult<(u64, Vec<u64>)> {
    if self.target_reg.is_arg() {
      return Err(SwppErrorKind::ArgRegAssign(self.target_reg.clone()));
    }
    let addr = self.addr_reg.read_value_with_state(state, reg_set)?;

    let size: u64 = self.size.into();

    if addr % size != 0 {
      return Err(SwppErrorKind::InvalidAlignment(addr, size));
    }
    let (val, base_cost) = if addr <= MEM_STACK_SIZE {
      (
        state.read_from_stack(addr, self.size)?,
        MEMORY_ACCESS_COST_STACK,
      )
    } else if addr >= HEAP_OFFSET {
      (
        state.read_from_heap(addr, self.size)?,
        MEMORY_ACCESS_COST_HEAP,
      )
    } else {
      return Err(SwppErrorKind::InvalidAddr(addr));
    };

    // Calculate heat cost
    let (heat_cost, affected_sectors) = if addr <= MEM_STACK_SIZE {
      state.cost_stack(addr, self.size)
    } else {
      state.cost_heap(addr, self.size)
    };

    let total_cost = base_cost + heat_cost;

    state.write_register_with_debt(reg_set, &self.target_reg, val)?;

    Ok((total_cost, affected_sectors))
  }
}

/// Store
#[derive(Debug, Clone, new)]
pub struct InstStore {
  val_reg: Arg,
  addr_reg: Arg,
  size: AccessSize,
}

impl InstStore {
  pub fn run(
    &self,
    state: &mut SwppState,
    reg_set: &mut SwppRegisterSet,
  ) -> SwppRawResult<(u64, Vec<u64>)> {
    let val = self.val_reg.read_value_with_state(state, reg_set)?;
    let addr = self.addr_reg.read_value_with_state(state, reg_set)?;

    let size: u64 = self.size.into();

    if addr % size != 0 {
      return Err(SwppErrorKind::InvalidAlignment(addr, size));
    }

    let base_cost = if addr <= MEM_STACK_SIZE {
      state.write_to_stack(addr, val, self.size)?;
      MEMORY_ACCESS_COST_STACK
    } else if addr >= HEAP_OFFSET {
      state.write_to_heap(addr, val, self.size)?;
      MEMORY_ACCESS_COST_HEAP
    } else {
      return Err(SwppErrorKind::InvalidAddr(addr));
    };

    // Calculate heat cost
    let (heat_cost, affected_sectors) = if addr <= MEM_STACK_SIZE {
      state.cost_stack(addr, self.size)
    } else {
      state.cost_heap(addr, self.size)
    };

    let total_cost = base_cost + heat_cost;

    Ok((total_cost, affected_sectors))
  }
}

/// Async Load
#[derive(Debug, Clone, new)]
pub struct InstAsyncLoad {
  target_reg: SwppRegisterName,
  addr_reg: Arg,
  size: AccessSize,
}

impl InstAsyncLoad {
  pub fn run(
    &self,
    state: &mut SwppState,
    reg_set: &mut SwppRegisterSet,
  ) -> SwppRawResult<(u64, Vec<u64>)> {
    if self.target_reg.is_arg() {
      return Err(SwppErrorKind::ArgRegAssign(self.target_reg.clone()));
    }
    let addr = self.addr_reg.read_value_with_state(state, reg_set)?;

    let size: u64 = self.size.into();

    if addr % size != 0 {
      return Err(SwppErrorKind::InvalidAlignment(addr, size));
    }
    let val = if addr <= MEM_STACK_SIZE {
      state.read_from_stack(addr, self.size)?
    } else if addr >= HEAP_OFFSET {
      state.read_from_heap(addr, self.size)?
    } else {
      return Err(SwppErrorKind::InvalidAddr(addr));
    };

    // Calculate heat cost and get affected sectors
    let (heat_cost, affected_sectors) = if addr <= MEM_STACK_SIZE {
      state.cost_stack(addr, self.size)
    } else {
      state.cost_heap(addr, self.size)
    };

    // Async load base cost is always 1, but creates debt
    let base_cost = 1;
    let debt = if addr <= MEM_STACK_SIZE { 36 } else { 60 };

    let total_cost = base_cost + heat_cost;

    // Overwriting the target should clear any previous outstanding debt first.
    state.write_register_with_debt(reg_set, &self.target_reg, val)?;

    // Then register the async-load debt for the newly written value.
    state.create_aload_debt(self.target_reg.clone(), debt);

    Ok((total_cost, affected_sectors))
  }
}

/// Unsigned Division
#[derive(Debug, Clone, new)]
pub struct InstUnsignedDivision {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstUnsignedDivision {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);
    let val = self.bw.read_u64(val1 / val2);

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

/// Signed Division
#[derive(Debug, Clone, new)]
pub struct InstSignedDivision {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstSignedDivision {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .sign_extend_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .sign_extend_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_i64(val1 / val2);

    let val = val as u64;

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

/// Unsigned Remainder
#[derive(Debug, Clone, new)]
pub struct InstUnsignedRemainder {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstUnsignedRemainder {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1 % val2);

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}
/// Signed Remainder
#[derive(Debug, Clone, new)]
pub struct InstSignedRemainder {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstSignedRemainder {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .sign_extend_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .sign_extend_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_i64(val1 % val2);

    let val = val as u64;

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

/// Multiplication
#[derive(Debug, Clone, new)]
pub struct InstMultiplication {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstMultiplication {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1.wrapping_mul(val2));

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstShiftLeft {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstShiftLeft {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1 << val2);

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstShiftRightLogical {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstShiftRightLogical {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1 >> val2);

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstShiftRightArithmetic {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstShiftRightArithmetic {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .sign_extend_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_i64(val1 >> val2);
    let val = val as u64;
    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstBitwiseAnd {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstBitwiseAnd {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    state.write_register_with_debt(reg_set, &self.target_reg, self.bw.read_u64(val1 & val2))
  }
}

#[derive(Debug, Clone, new)]
pub struct InstBitwiseOr {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstBitwiseOr {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    state.write_register_with_debt(reg_set, &self.target_reg, self.bw.read_u64(val1 | val2))
  }
}

#[derive(Debug, Clone, new)]
pub struct InstBitwiseXor {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstBitwiseXor {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    state.write_register_with_debt(reg_set, &self.target_reg, self.bw.read_u64(val1 ^ val2))
  }
}

#[derive(Debug, Clone, new)]
pub struct InstEAdd {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstEAdd {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<u64> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1.wrapping_add(val2));
    let cost = if val % 2 == 0 {
      BASIC_COST_ARITH_INT_ADDSUB_RIGHT
    } else {
      BASIC_COST_ARITH_INT_ADDSUB_WRONG
    };

    state.write_register_with_debt(reg_set, &self.target_reg, val)?;

    Ok(cost)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstOAdd {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstOAdd {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<u64> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1.wrapping_add(val2));
    let cost = if val % 2 == 1 {
      BASIC_COST_ARITH_INT_ADDSUB_RIGHT
    } else {
      BASIC_COST_ARITH_INT_ADDSUB_WRONG
    };

    state.write_register_with_debt(reg_set, &self.target_reg, val)?;

    Ok(cost)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstESub {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstESub {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<u64> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1.wrapping_sub(val2));
    let cost = if val % 2 == 0 {
      BASIC_COST_ARITH_INT_ADDSUB_RIGHT
    } else {
      BASIC_COST_ARITH_INT_ADDSUB_WRONG
    };

    state.write_register_with_debt(reg_set, &self.target_reg, val)?;

    Ok(cost)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstOSub {
  reg1: Arg,
  reg2: Arg,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstOSub {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<u64> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = self.bw.read_u64(val1.wrapping_sub(val2));
    let cost = if val % 2 == 1 {
      BASIC_COST_ARITH_INT_ADDSUB_RIGHT
    } else {
      BASIC_COST_ARITH_INT_ADDSUB_WRONG
    };

    state.write_register_with_debt(reg_set, &self.target_reg, val)?;

    Ok(cost)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstComparison {
  reg1: Arg,
  reg2: Arg,
  cond: ICMP,
  target_reg: SwppRegisterName,
  bw: BitWidth,
}

impl InstComparison {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let val1 = self
      .bw
      .read_u64(self.reg1.read_value_with_state(state, reg_set)?);
    let val2 = self
      .bw
      .read_u64(self.reg2.read_value_with_state(state, reg_set)?);

    let val = match self.bw {
      BitWidth::Bit => {
        let val1 = val1 != 0;
        let val2 = val2 != 0;
        self.cond.compare_bit(val1, val2) as u64
      }
      BitWidth::Byte => {
        let val1 = val1 as u8;
        let val2 = val2 as u8;
        self.cond.compare_u8(val1, val2) as u64
      }
      BitWidth::Short => {
        let val1 = val1 as u16;
        let val2 = val2 as u16;
        self.cond.compare_u16(val1, val2) as u64
      }
      BitWidth::Quad => {
        let val1 = val1 as u32;
        let val2 = val2 as u32;
        self.cond.compare_u32(val1, val2) as u64
      }
      BitWidth::Full => {
        let val1 = val1 as u64;
        let val2 = val2 as u64;
        self.cond.compare_u64(val1, val2) as u64
      }
    };

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

#[derive(Debug, Clone, new)]
pub struct InstTernary {
  false_reg: Arg,
  true_reg: Arg,
  cond_reg: Arg,
  target_reg: SwppRegisterName,
  // bw : BitWidth,
}

impl InstTernary {
  pub fn run(&self, state: &mut SwppState, reg_set: &mut SwppRegisterSet) -> SwppRawResult<()> {
    let cond = self.cond_reg.read_value_with_state(state, reg_set)?;
    let val = match cond {
      0 => self.false_reg.read_value_with_state(state, reg_set)?,
      1 => self.true_reg.read_value_with_state(state, reg_set)?,
      _ => return Err(SwppErrorKind::InvalidCondVal(cond)),
    };

    state.write_register_with_debt(reg_set, &self.target_reg, val)
  }
}

/// Only for predefiend function read
#[derive(Debug, Clone, new)]
pub struct InstStdRead {}

impl InstStdRead {
  pub fn run(&self) -> SwppRawResult<u64> {
    let val: u64 = read!();

    Ok(val)
  }
}

/// Only for pre-defiend function write
#[derive(Debug, Clone, new)]
pub struct InstStdWrite {}

impl InstStdWrite {
  pub fn run(&self, reg_set: &SwppRegisterSet) -> SwppRawResult<()> {
    let arg = SwppRegisterName::Argument(1);
    let write_val = reg_set.read_register_word(&arg)?;
    println!("{write_val}");
    Ok(())
  }
}
