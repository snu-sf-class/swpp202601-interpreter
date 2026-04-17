use std::{
  fs::File,
  io::{Read, Write},
  str::from_utf8,
};

use swpp_interpreter::{
  parser::total_program_parser,
  program::{SwppProgram, SwppState},
};

#[cfg(not(feature = "log"))]
use swpp_interpreter::logger::DummyLogger;
#[cfg(feature = "log")]
use swpp_interpreter::logger::FileLogger;

fn main() {
  let asm_path = std::env::args().nth(1).expect("no Assembly file link");

  let cost_path = std::env::args().nth(2).expect("no Cost file link");

  let mut asm_file = File::options()
    .read(true)
    .open(&asm_path)
    .expect(&format!("Fail to open Assembly file : {}", asm_path));

  let mut buf = [0; 50000];

  let size = asm_file.read(&mut buf).expect("Fail to read assembly file");

  if size >= 50000 {
    panic!("Assembly longer than 50000 bytes is not supported")
  }

  let asm = from_utf8(&buf).expect("There is invalid character in assembly");

  #[cfg(feature = "log")]
  let logger = FileLogger::new("./swpp-interpreter-basic.log");
  #[cfg(not(feature = "log"))]
  let logger = DummyLogger::new();

  let mut program = SwppProgram::new(SwppState::new(total_program_parser(asm)), logger);

  program.run().unwrap_or_else(|err| {
    println!(
      "Your assembly fails with following Error\n{}",
      err.to_string()
    );
    panic!()
  });

  let mut cost_file = File::options()
    .write(true)
    .create(true)
    .open(&cost_path)
    .expect(&format!("Fail to open Cost file : {}", asm_path));

  let final_cost = format!("Final Cost : {}", program.total_cost());

  cost_file.write(final_cost.as_bytes()).unwrap();
}
