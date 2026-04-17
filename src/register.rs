use crate::{common::MEM_STACK_SIZE, error::SwppRawResult};

/// This struct represents the local state of register
#[derive(Debug, Clone)]
pub struct SwppRegisterSet {
  /// 32 64-bit general registers
  general: [u64; 32],
  /// stack pointer (set to MEM_STACK_SIZE at the beginning of the program)
  sp: u64,
  /// 16 argument registers
  arg: [u64; 16],
}

impl Default for SwppRegisterSet {
  fn default() -> Self {
    Self {
      general: Default::default(),
      sp: MEM_STACK_SIZE,
      arg: Default::default(),
    }
  }
}

impl SwppRegisterSet {
  pub fn read_register_word(&self, rname: &SwppRegisterName) -> SwppRawResult<u64> {
    match rname {
      SwppRegisterName::Gen(idx) => Ok(self.general[*idx - 1]),
      SwppRegisterName::StackPointer => Ok(self.sp),
      SwppRegisterName::Argument(idx) => Ok(self.arg[*idx - 1]),
    }
  }

  pub fn write_register_word(&mut self, rname: &SwppRegisterName, val: u64) -> SwppRawResult<()> {
    match rname {
      SwppRegisterName::Gen(idx) => self.general[*idx - 1] = val,
      SwppRegisterName::StackPointer => self.sp = val,
      SwppRegisterName::Argument(idx) => self.arg[*idx - 1] = val,
    };
    Ok(())
  }

  pub fn set_arg_register(&mut self, args: &Vec<u64>) {
    assert!(args.len() <= 16);
    for (i, arg) in args.iter().enumerate() {
      self.arg[i] = *arg;
    }
  }

  pub fn get_register_word_mut(&mut self, rname: &SwppRegisterName) -> SwppRawResult<&mut u64> {
    match rname {
      SwppRegisterName::Gen(idx) => Ok(&mut self.general[*idx - 1]),
      SwppRegisterName::StackPointer => Ok(&mut self.sp),
      SwppRegisterName::Argument(idx) => Ok(&mut self.arg[*idx - 1]),
    }
  }

  pub fn print_gen_register(&self) -> String {
    format!("r : {:?}", self.general)
  }
}

/// Index for accessing registers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SwppRegisterName {
  ///General (0~31)
  Gen(usize),
  /// sp
  StackPointer,
  /// arg (0~15)
  Argument(usize),
}

impl ToString for SwppRegisterName {
  fn to_string(&self) -> String {
    match self {
      SwppRegisterName::Gen(idx) => format!("r{}", idx),
      SwppRegisterName::StackPointer => String::from("sp"),
      SwppRegisterName::Argument(idx) => format!("arg{}", idx),
    }
  }
}

impl SwppRegisterName {
  pub fn is_arg(&self) -> bool {
    match self {
      SwppRegisterName::Argument(_) => true,
      _ => false,
    }
  }
}
