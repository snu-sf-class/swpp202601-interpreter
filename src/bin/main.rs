use std::{
  fs::{read_to_string, File},
  io::Write,
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

  let asm = read_to_string(&asm_path).expect(&format!("Fail to read Assembly file : {}", asm_path));

  #[cfg(feature = "log")]
  let logger = FileLogger::new("./swpp-interpreter-basic.log");
  #[cfg(not(feature = "log"))]
  let logger = DummyLogger::new();

  let mut program = SwppProgram::new(SwppState::new(total_program_parser(&asm)), logger);

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
    .truncate(true)
    .open(&cost_path)
    .expect(&format!("Fail to open Cost file : {}", cost_path));

  let final_cost = format!("Final Cost : {}", program.total_cost());

  cost_file.write(final_cost.as_bytes()).unwrap();
}
