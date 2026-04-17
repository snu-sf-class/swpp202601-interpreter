use std::collections::HashMap;

use crate::{error::SwppRawResult, program::SwppBlock};

/// *Function* is a set of named blocks
#[derive(Debug, Clone)]
pub struct SwppFunction {
  /// Function name
  fname: String,
  /// A number of arguments
  nargs: u64,
  /// starting block of the function
  entry: String,
  /// A map of all blocks in the function
  block_map: HashMap<String, SwppBlock>,
  /// A index map of all blocks in the function
  block_index_map: HashMap<String, u64>,
}

impl SwppFunction {
  pub fn new(fname: String, nargs: u64, blocks: Vec<SwppBlock>) -> Self {
    let entry = blocks
      .first()
      .expect("Empty Function not allowed")
      .block_name
      .clone();
    let block_map = blocks
      .clone()
      .into_iter()
      .map(|b| (b.block_name.clone(), b))
      .collect();
    let block_index_map = blocks
      .clone()
      .iter()
      .enumerate()
      .map(|(i, b)| (b.block_name.clone(), i as u64))
      .collect();
    Self {
      fname,
      nargs,
      entry,
      block_map,
      block_index_map,
    }
  }

  pub fn get_block_index(&self, block_name: &str) -> SwppRawResult<u64> {
    self.block_index_map.get(block_name).cloned().ok_or(
      crate::error::SwppErrorKind::UnknownBlockName(block_name.to_owned()),
    )
  }

  pub fn get_entry_block_name(&self) -> String {
    self.entry.clone()
  }

  pub fn get_block(&self, block_name: &str) -> SwppRawResult<&SwppBlock> {
    self
      .block_map
      .get(block_name)
      .ok_or(crate::error::SwppErrorKind::UnknownBlockName(
        block_name.to_owned(),
      ))
  }

  pub fn nargs(&self) -> u64 {
    self.nargs
  }

  pub fn fname(&self) -> String {
    self.fname.clone()
  }
}
