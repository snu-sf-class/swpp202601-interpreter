use crate::register::SwppRegisterName;

pub type SwppResult<T> = Result<T, SwppError>;
pub type SwppRawResult<T> = Result<T, SwppErrorKind>;

/// Kinds of errors that can occur during the execution of the program
/// You can refer to the desc function for the description of each error kind
#[derive(Debug, Clone)]
pub enum SwppErrorKind {
  /// There is no main function in the program
  NoMainFn,
  /// Call of a function that isn't declared
  UnknownFnName(String),
  /// Access to an undeclared block
  UnknownBlockName(String),
  /// Block is not well formed
  IllFormedBlock(String),
  /// Wrong register name is used
  WrongRegisterName(SwppRegisterName),
  /// Vector register is used where a non-vector register is expected
  ExpectVecReg(SwppRegisterName),
  /// Non-vector register is used where a vector register is expected
  ExpectNonVecReg(SwppRegisterName),
  /// Attempt to assign a value directly to an argument register
  ArgRegAssign(SwppRegisterName),
  /// Invalid value used for condition register
  InvalidCondVal(u64),
  /// Function call results in a crash, with the error statement provided
  FunctionCallCrash(String, String),
  /// Wrong number of arguments provided to a function call
  WrongArgNum(String, u64, u64),
  /// Attempt to assign a value to a function that returns nothing
  AssignNoValue(String),
  /// Main function is recursively called
  RecursiveMainCall,
  /// Unallowed recursive call
  InvalidRecursiveCall(String, String),
  /// Assert failed
  AssertionFailed(u64, u64),
  /// Invalid heap allocation size
  InvalidHeapAllocSize(u64),
  /// Heap memory is exhausted
  NOMEMHEAP,
  /// Wrong address is accessed
  InvalidAddr(u64),
  /// Wrong alignment for memory access
  InvalidAlignment(u64, u64),
  /// read/write fails
  IOFails,
  /// Invalid value is read from stdstream
  InvalidIOValue(String),
  SubtractOverFlow,
}

impl SwppErrorKind {
  pub fn desc(&self) -> String {
    match self {
            SwppErrorKind::NoMainFn => String::from("Main Function does not exists"),
            SwppErrorKind::UnknownFnName(fname) => format!("Function {} isn't declared", &fname),
            SwppErrorKind::UnknownBlockName(bname) => format!("Block {} isn't declared", &bname),
            SwppErrorKind::IllFormedBlock(bname) => format!("Block {} is not well formed", &bname),
            SwppErrorKind::WrongRegisterName(rname) => {
                format!("Register {} doesn't exist in system", rname.to_string())
            }
            SwppErrorKind::ExpectVecReg(rname) => {
                format!("Expected Vector Register but find {}", rname.to_string())
            }
            SwppErrorKind::ArgRegAssign(rname) => format!(
                "You cannot assign the value directly to the argument register {}",
                rname.to_string()
            ),
            SwppErrorKind::ExpectNonVecReg(rname) => format!(
                "Expected non-Vector Register but find {}",
                rname.to_string()
            ),
            SwppErrorKind::InvalidCondVal(val) => {
                format!("Condition register must have 0 or 1 but you use {}", val)
            }
            SwppErrorKind::FunctionCallCrash(fname, error_stmt) => format!(
                "While running {fname}, following error occurs \n-------------------------------------------------\n {error_stmt} \n-------------------------------------------------\n"
            ),
            SwppErrorKind::WrongArgNum(fname, right, wrong) => {
                format!("Function {fname} takes {right} arguments but you give {wrong}")
            }
            SwppErrorKind::AssignNoValue(fname) => format!(
                "{fname} returns nothing."
            ),
            SwppErrorKind::RecursiveMainCall => {
                String::from("Main function cannot be recursively called")
            }
            SwppErrorKind::InvalidRecursiveCall(fname, context) => {
                format!("You cannot recursively call {fname} in the function {context}")
            }
            SwppErrorKind::AssertionFailed(rhs, lhs) => format!(
                "Assertion Failed. Right side has value {rhs:?} while left side has value {lhs:?}"
            ),
            SwppErrorKind::InvalidHeapAllocSize(size) => {
                format!("Size for heap allocation {size} should be non-zero and multiple of 8")
            }
            SwppErrorKind::NOMEMHEAP => {
                "Heap Memory is actually limited with size of 2^64 bytes.".to_string()
            }
            SwppErrorKind::InvalidAddr(addr) => {
                format!("Error occurs while trying to access adress {addr}")
            }
            SwppErrorKind::InvalidAlignment(addr, size) => {
                format!("{addr} should be multiple of {size}")
            }
            SwppErrorKind::IOFails =>{
                String::from("fail to read or write to stdstream")
            }
            SwppErrorKind::InvalidIOValue(input) => {
                format!("Read Invalid value {input}")
            }
            SwppErrorKind::SubtractOverFlow => {
                "Overflow occurs while subtract".to_string()
            },
        }
  }
}

#[derive(Debug)]
pub struct SwppError {
  kind: SwppErrorKind,
  loc: u64,
}

impl SwppError {
  pub fn new(kind: SwppErrorKind, loc: u64) -> Self {
    Self { kind, loc }
  }

  pub fn get_kind(&self) -> SwppErrorKind {
    self.kind.clone()
  }
}

impl ToString for SwppError {
  fn to_string(&self) -> String {
    format!("{} : line {}", self.kind.desc(), self.loc)
  }
}
