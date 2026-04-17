use std::{fs::File, io::BufWriter};

use crate::program::SwppContext;

pub trait SwppLogger {
  fn log(&mut self, line_num: u64, inst: &str, cost: u64, total_cost: u64, ctxt: &SwppContext);
  fn enter_fn(&mut self);
  fn exit_fn(&mut self);
}

#[derive(Debug)]
pub struct DummyLogger {}

impl DummyLogger {
  pub fn new() -> Self {
    Self {}
  }
}

impl SwppLogger for DummyLogger {
  fn log(
    &mut self,
    _line_num: u64,
    _inst: &str,
    _cost: u64,
    _total_cost: u64,
    _ctxt: &SwppContext,
  ) {
  }
  fn enter_fn(&mut self) {}
  fn exit_fn(&mut self) {}
}

#[derive(Debug)]
pub struct FileLogger {
  idx: u64,
  cur_tab: u64,
  base_log_writer: BufWriter<File>,
}

impl FileLogger {
  pub fn new(base_log_path: &str) -> Self {
    use std::io::Write;

    let base_log_writer = File::options()
      .write(true)
      .create(true)
      .truncate(true)
      .open(base_log_path)
      .expect(&format!("Fail to open log file : {}", base_log_path));

    let mut base_log_writer = BufWriter::new(base_log_writer);

    let line = format!(
      "{:^6}|{:^20}|{:^8}|{:^4}|{:^10}|{:30}\n",
      "Index", "InstructionKind", "LineNum", "Cost", "TotalCost", "CurrentScope",
    );

    base_log_writer
      .write(line.as_bytes())
      .expect("Logging Error");

    Self {
      idx: 0,
      cur_tab: 0,
      base_log_writer,
    }
  }
}

impl SwppLogger for FileLogger {
  fn log(&mut self, line_num: u64, inst: &str, cost: u64, total_cost: u64, ctxt: &SwppContext) {
    use std::io::Write;

    let line = format!(
      "{:6}|{:^20}|{:8}|{:4}|{:10}|{:30}\n",
      self.idx,
      inst,
      line_num,
      cost,
      total_cost,
      ctxt.get_fname(),
    );

    self
      .base_log_writer
      .write(line.as_bytes())
      .expect("Logging Error");
    self.idx += 1;
  }

  fn enter_fn(&mut self) {
    self.cur_tab += 1;
  }

  fn exit_fn(&mut self) {
    self.cur_tab -= 1;
  }
}
