use crate::{
  error::SwppRawResult,
  logger::SwppLogger,
  program::SwppState,
  register::{SwppRegisterName, SwppRegisterSet},
};

pub const MEM_STACK_SIZE: u64 = 102400;
pub const MAIN_NAME: &str = "main";
pub const INTERNAL_ERROR_MSG: &str =
  "Internal Logic Error. If students find this bug, please tell TAs.";
pub const HEAP_OFFSET: u64 = 204800;
pub const MAX_HEAP_SIZE: u64 = u64::MAX - HEAP_OFFSET;
pub const NULL_ADDR: u64 = 153600;

// Memory access costs
pub const MEMORY_ACCESS_COST_STACK: u64 = 30;
pub const MEMORY_ACCESS_COST_HEAP: u64 = 50;

#[derive(Debug, Clone, Copy)]
pub enum AccessSize {
  One,
  Two,
  Four,
  Eight,
}

impl From<u64> for AccessSize {
  fn from(value: u64) -> Self {
    match value {
      1 => Self::One,
      2 => Self::Two,
      4 => Self::Four,
      8 => Self::Eight,
      _ => unreachable!(),
    }
  }
}

impl From<AccessSize> for usize {
  fn from(value: AccessSize) -> Self {
    match value {
      AccessSize::One => 1,
      AccessSize::Two => 2,
      AccessSize::Four => 4,
      AccessSize::Eight => 8,
    }
  }
}
impl From<AccessSize> for u64 {
  fn from(value: AccessSize) -> Self {
    match value {
      AccessSize::One => 1,
      AccessSize::Two => 2,
      AccessSize::Four => 4,
      AccessSize::Eight => 8,
    }
  }
}

impl AccessSize {
  pub fn is_four_phobia(&self) -> bool {
    u64::from(*self) == 4
  }

  pub fn add_cost_if_four_phobia<L: SwppLogger>(
    &self,
    state: &mut SwppState,
    logger: &mut L,
    loc: u64,
    inst_name: &str,
  ) -> u64 {
    if self.is_four_phobia() {
      let cost = BASIC_COST_MISC_FOUR_PHOBIA;
      state.add_cost(cost);
      let log_name = format!("{}-4phobia", inst_name);
      logger.log(loc, &log_name, cost, state.get_cost(), state.get_context());
      cost
    } else {
      0
    }
  }
}

#[derive(Debug, Clone)]
pub enum BitWidth {
  Bit,   //1
  Byte,  //8
  Short, //16
  Quad,  //32
  Full,  //64
}

impl BitWidth {
  pub fn read_u64(&self, val: u64) -> u64 {
    let bit: u64 = self.clone().into();
    let shift = 64 - bit;
    if shift == 0 {
      val
    } else {
      let mask = (1u64 << bit) - 1;
      val & mask
    }
  }

  pub fn read_i64(&self, val: i64) -> i64 {
    let bit: u64 = self.clone().into();
    let shift = 64 - bit;
    if shift == 0 {
      val
    } else {
      let mask = (1i64 << bit) - 1;
      val & mask
    }
  }

  pub fn sign_extend_u64(&self, val: u64) -> i64 {
    let bit: u64 = self.clone().into();
    let shift = 64 - bit;
    let masked = self.read_u64(val);
    if shift == 0 {
      masked as i64
    } else {
      ((masked << shift) as i64) >> shift
    }
  }
}

impl From<u64> for BitWidth {
  fn from(value: u64) -> Self {
    match value {
      1 => Self::Bit,
      8 => Self::Byte,
      16 => Self::Short,
      32 => Self::Quad,
      64 => Self::Full,
      _ => panic!(),
    }
  }
}

impl From<BitWidth> for usize {
  fn from(value: BitWidth) -> Self {
    match value {
      BitWidth::Bit => 1,
      BitWidth::Byte => 8,
      BitWidth::Short => 16,
      BitWidth::Quad => 32,
      BitWidth::Full => 64,
    }
  }
}
impl From<BitWidth> for u64 {
  fn from(value: BitWidth) -> Self {
    match value {
      BitWidth::Bit => 1,
      BitWidth::Byte => 8,
      BitWidth::Short => 16,
      BitWidth::Quad => 32,
      BitWidth::Full => 64,
    }
  }
}

#[derive(Debug, Clone)]
pub enum ICMP {
  Eq,
  Ne,
  Ugt,
  Uge,
  Ult,
  Ule,
  Sgt,
  Sge,
  Slt,
  Sle,
}

impl ICMP {
  pub fn compare_u64(&self, rhs: u64, lhs: u64) -> bool {
    let rhs_s: i64 = rhs as i64;
    let lhs_s: i64 = lhs as i64;

    match self {
      ICMP::Eq => rhs == lhs,
      ICMP::Ne => rhs != lhs,
      ICMP::Ugt => rhs > lhs,
      ICMP::Uge => rhs >= lhs,
      ICMP::Ult => rhs < lhs,
      ICMP::Ule => rhs <= lhs,
      ICMP::Sgt => rhs_s > lhs_s,
      ICMP::Sge => rhs_s >= lhs_s,
      ICMP::Slt => rhs_s < lhs_s,
      ICMP::Sle => rhs_s <= lhs_s,
    }
  }

  pub fn compare_u32(&self, rhs: u32, lhs: u32) -> bool {
    let rhs_s: i32 = rhs as i32;
    let lhs_s: i32 = lhs as i32;

    match self {
      ICMP::Eq => rhs == lhs,
      ICMP::Ne => rhs != lhs,
      ICMP::Ugt => rhs > lhs,
      ICMP::Uge => rhs >= lhs,
      ICMP::Ult => rhs < lhs,
      ICMP::Ule => rhs <= lhs,
      ICMP::Sgt => rhs_s > lhs_s,
      ICMP::Sge => rhs_s >= lhs_s,
      ICMP::Slt => rhs_s < lhs_s,
      ICMP::Sle => rhs_s <= lhs_s,
    }
  }

  pub fn compare_u16(&self, rhs: u16, lhs: u16) -> bool {
    let rhs_s: i16 = rhs as i16;
    let lhs_s: i16 = lhs as i16;

    match self {
      ICMP::Eq => rhs == lhs,
      ICMP::Ne => rhs != lhs,
      ICMP::Ugt => rhs > lhs,
      ICMP::Uge => rhs >= lhs,
      ICMP::Ult => rhs < lhs,
      ICMP::Ule => rhs <= lhs,
      ICMP::Sgt => rhs_s > lhs_s,
      ICMP::Sge => rhs_s >= lhs_s,
      ICMP::Slt => rhs_s < lhs_s,
      ICMP::Sle => rhs_s <= lhs_s,
    }
  }

  pub fn compare_u8(&self, rhs: u8, lhs: u8) -> bool {
    let rhs_s: i8 = rhs as i8;
    let lhs_s: i8 = lhs as i8;

    match self {
      ICMP::Eq => rhs == lhs,
      ICMP::Ne => rhs != lhs,
      ICMP::Ugt => rhs > lhs,
      ICMP::Uge => rhs >= lhs,
      ICMP::Ult => rhs < lhs,
      ICMP::Ule => rhs <= lhs,
      ICMP::Sgt => rhs_s > lhs_s,
      ICMP::Sge => rhs_s >= lhs_s,
      ICMP::Slt => rhs_s < lhs_s,
      ICMP::Sle => rhs_s <= lhs_s,
    }
  }

  pub fn compare_bit(&self, rhs: bool, lhs: bool) -> bool {
    let rhs = rhs as u8;
    let lhs = lhs as u8;

    match self {
      ICMP::Eq => rhs == lhs,
      ICMP::Ne => rhs != lhs,
      ICMP::Ugt => rhs > lhs,
      ICMP::Uge => rhs >= lhs,
      ICMP::Ult => rhs < lhs,
      ICMP::Ule => rhs <= lhs,
      ICMP::Sgt => rhs > lhs,
      ICMP::Sge => rhs >= lhs,
      ICMP::Slt => rhs < lhs,
      ICMP::Sle => rhs <= lhs,
    }
  }
}

/// Operands can be either a register or a constant value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
  Reg(SwppRegisterName),
  Const(u64),
}

// Basic Instruction Costs (make them constants to avoid using a magic number in the code)
pub const BASIC_COST_CONTROL_RET: u64 = 1;
pub const BASIC_COST_CONTROL_UBRANCH: u64 = 30;
pub const BASIC_COST_CONTROL_BRANCH_TRUE: u64 = 90;
pub const BASIC_COST_CONTROL_BRANCH_FALSE: u64 = 30;
pub const FORWARD_BRANCH_COST_MULTIPLIER: f64 = 1.5;
pub const BASIC_COST_CONTROL_SWITCH: u64 = 60;
pub const BASIC_COST_CONTROL_FNCALL: u64 = 30;
pub const BASIC_COST_MEM_HEAP_OP: u64 = 150;
pub const BASIC_COST_ARITH_INT_MULDIV: u64 = 2;
pub const BASIC_COST_ARITH_INT_SHIFT: u64 = 10;
pub const BASIC_COST_ARITH_BITWISE: u64 = 10;
pub const BASIC_COST_ARITH_INT_ADDSUB_RIGHT: u64 = 10;
pub const BASIC_COST_ARITH_INT_ADDSUB_WRONG: u64 = 20;
pub const BASIC_COST_ARITH_INT_ADDSUB_FMA: u64 = 0;
pub const BASIC_COST_ARITH_CMP: u64 = 3;
pub const BASIC_COST_ARITH_SELECT: u64 = 3;
pub const BASIC_COST_MISC_FOUR_PHOBIA: u64 = 10;

impl Arg {
  pub fn read_value(&self, reg_set: &SwppRegisterSet) -> SwppRawResult<u64> {
    match self {
      Arg::Reg(reg) => reg_set.read_register_word(reg),
      Arg::Const(val) => Ok(*val),
    }
  }

  pub fn read_value_with_state(
    &self,
    state: &mut SwppState,
    reg_set: &SwppRegisterSet,
  ) -> SwppRawResult<u64> {
    match self {
      Arg::Reg(reg) => state.read_register_with_debt(reg_set, reg),
      Arg::Const(val) => Ok(*val),
    }
  }

  /// This function is required for implementing 4-Phobia
  pub fn is_const_four(&self) -> bool {
    match self {
      Arg::Const(val) => *val == 4,
      _ => false,
    }
  }

  pub fn add_cost_if_four_phobia<L: SwppLogger>(
    &self,
    state: &mut SwppState,
    logger: &mut L,
    loc: u64,
    inst_name: &str,
  ) -> u64 {
    if self.is_const_four() {
      let cost = BASIC_COST_MISC_FOUR_PHOBIA;
      state.add_cost(cost);
      let log_name = format!("{}-4phobia", inst_name);
      logger.log(loc, &log_name, cost, state.get_cost(), state.get_context());
      cost
    } else {
      0
    }
  }
}
