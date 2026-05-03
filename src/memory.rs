use std::{collections::BTreeMap, ops::Bound};

use crate::{
  common::{AccessSize, HEAP_OFFSET, MAX_HEAP_SIZE, MEM_STACK_SIZE, NULL_ADDR},
  error::{SwppErrorKind, SwppRawResult},
};

// Heat system constants
const HEAT_INCREASE_PER_ACCESS: u64 = 400;
const HEAT_CAP_PER_SECTOR: u64 = 2000;
const HEAT_COST_DIVISOR: u64 = 10;
const SECTOR_SIZE: u64 = 8;

/// Struct to abstract entire memory, which consists of stack and heap
pub struct SwppMemory {
  /// stack
  stack: [u8; MEM_STACK_SIZE as usize],
  heap: SwppSimpleHeap,
  /// Heat tracking for heap sectors (sector_index -> heat)
  heap_heat: BTreeMap<u64, u64>,
}

impl SwppMemory {
  pub fn get_max_heap_size(&self) -> u64 {
    self.heap.max_size
  }

  pub fn new() -> Self {
    Self {
      stack: [0; MEM_STACK_SIZE as usize],
      heap: SwppSimpleHeap::new(),
      heap_heat: BTreeMap::new(),
    }
  }

  pub fn malloc(&mut self, size: u64) -> SwppRawResult<u64> {
    self.heap.malloc(size)
  }

  pub fn free(&mut self, addr: u64) -> SwppRawResult<()> {
    if addr == NULL_ADDR {
      return Ok(());
    }
    if addr < HEAP_OFFSET {
      return Err(SwppErrorKind::InvalidAddr(addr));
    }

    // Get the size of the freed allocation to reset its heat
    if let Some(alloc_size) = self.heap.get_allocation_size(addr) {
      // Reset heat for all sectors in this allocation
      let start_sector = addr / SECTOR_SIZE;
      let end_sector = (addr + alloc_size + SECTOR_SIZE - 1) / SECTOR_SIZE;
      for sector_idx in start_sector..end_sector {
        self.heap_heat.remove(&sector_idx);
      }
    }

    self.heap.free(addr)
  }

  pub fn read_from_stack(&self, addr: u64, size: AccessSize) -> SwppRawResult<u64> {
    let mut byte_arr = [0; 8];

    for i in 0..size.into() {
      byte_arr[i] = self
        .stack
        .get(addr as usize + i)
        .ok_or(SwppErrorKind::InvalidAddr(addr as u64))?
        .to_owned();
    }
    Ok(u64::from_le_bytes(byte_arr))
  }

  pub fn write_to_stack(&mut self, addr: u64, val: u64, size: AccessSize) -> SwppRawResult<()> {
    let val_bytes = val.to_le_bytes();

    for i in 0..size.into() {
      let target_mem = self
        .stack
        .get_mut(addr as usize + i)
        .ok_or(SwppErrorKind::InvalidAddr(addr as u64))?;
      *target_mem = val_bytes[i];
    }

    Ok(())
  }

  pub fn read_from_heap(&self, addr: u64, size: AccessSize) -> SwppRawResult<u64> {
    self.heap.read(addr, size)
  }

  pub fn write_to_heap(&mut self, addr: u64, val: u64, size: AccessSize) -> SwppRawResult<()> {
    self.heap.write(addr, val, size)
  }

  pub fn print_memory(&self) -> String {
    format!(
      "Stack : {:?}\nHeap:{}",
      self.stack,
      self.heap.print_heap_memory()
    )
  }

  /// Calculate heat-reduced sectors affected by an access of given bandwidth
  fn get_affected_sectors(addr: u64, bandwidth: u64) -> Vec<u64> {
    let direct_sector = addr / SECTOR_SIZE;
    // Range calculation: bandwidth 1→0, 2→1, 4→2, 8→4
    let range = bandwidth / 2;
    let mut sectors = Vec::new();

    // Add sectors in range [direct - range, direct + range]
    for offset in 0..=range {
      if offset == 0 {
        sectors.push(direct_sector);
      } else {
        if direct_sector >= offset {
          sectors.push(direct_sector - offset);
        }
        sectors.push(direct_sector + offset);
      }
    }

    sectors.sort_unstable();
    sectors.dedup();
    sectors
  }

  /// Heap heat should stay inside the owning allocation, even if the neighboring
  /// sector belongs to a different malloc block.
  fn get_affected_heap_sectors(
    addr: u64,
    bandwidth: u64,
    allocation_start: u64,
    allocation_size: u64,
  ) -> Vec<u64> {
    let min_sector = allocation_start / SECTOR_SIZE;
    let max_sector = (allocation_start + allocation_size - 1) / SECTOR_SIZE;

    Self::get_affected_sectors(addr, bandwidth)
      .into_iter()
      .filter(|sector| *sector >= min_sector && *sector <= max_sector)
      .collect()
  }

  /// Apply heat to affected sectors and return the extra cost from the directly accessed sector.
  fn apply_heat_and_cost(
    heat_map: &mut BTreeMap<u64, u64>,
    cost_sector: u64,
    affected_sectors: &[u64],
  ) -> u64 {
    let extra_cost = heat_map.get(&cost_sector).copied().unwrap_or(0) / HEAT_COST_DIVISOR;

    for &sector_idx in affected_sectors {
      let current_heat = *heat_map.get(&sector_idx).unwrap_or(&0);
      let new_heat = (current_heat + HEAT_INCREASE_PER_ACCESS).min(HEAT_CAP_PER_SECTOR);
      heat_map.insert(sector_idx, new_heat);
    }

    extra_cost
  }

  /// Stack accesses no longer participate in the heat model.
  pub fn cost_stack(&mut self, _addr: u64, _size: AccessSize) -> (u64, Vec<u64>) {
    (0, Vec::new())
  }

  /// Cost of accessing heap with given address and size (includes heat penalty and applies heat)
  /// Returns (extra_cost, affected_sectors)
  pub fn cost_heap(&mut self, addr: u64, size: AccessSize) -> (u64, Vec<u64>) {
    let bandwidth = u64::from(size);
    let (allocation_start, allocation_size) = self
      .heap
      .get_allocation_bounds(addr)
      .expect("heap heat must be computed only for valid heap addresses");
    let affected_sectors =
      Self::get_affected_heap_sectors(addr, bandwidth, allocation_start, allocation_size);
    let cost_sector = addr / SECTOR_SIZE;
    let extra_cost = Self::apply_heat_and_cost(&mut self.heap_heat, cost_sector, &affected_sectors);
    (extra_cost, affected_sectors)
  }

  /// Cool down all sectors by reducing heat based on instruction cost, optionally excluding specific sectors
  pub fn cool_down(&mut self, instruction_cost: u64, exclude_sectors: Option<&[u64]>) {
    let exclude_set =
      exclude_sectors.map(|sectors| sectors.iter().collect::<std::collections::HashSet<_>>());

    // Cool down heap heat
    for (&sector_idx, heat) in self.heap_heat.iter_mut() {
      if exclude_set
        .as_ref()
        .map_or(true, |set| !set.contains(&sector_idx))
      {
        *heat = heat.saturating_sub(instruction_cost);
      }
    }
    // Remove sectors with zero heat
    self.heap_heat.retain(|_, &mut heat| heat > 0);
  }
}

struct SwppSimpleHeap {
  memory: BTreeMap<u64, Vec<u8>>,
  top_addr: u64,
  max_size: u64,
  cur_size: u64,
}

impl SwppSimpleHeap {
  fn new() -> Self {
    Self {
      memory: BTreeMap::new(),
      top_addr: HEAP_OFFSET,
      max_size: 0,
      cur_size: 0,
    }
  }

  fn print_heap_memory(&self) -> String {
    format!("{:?}", self.memory)
  }

  fn malloc(&mut self, size: u64) -> SwppRawResult<u64> {
    if size + self.top_addr > MAX_HEAP_SIZE {
      return Err(SwppErrorKind::NOMEMHEAP);
    }

    self.memory.insert(self.top_addr, vec![0; size as usize]);

    let old_addr = self.top_addr;
    self.top_addr += size;

    self.cur_size += size;
    self.max_size = self.max_size.max(self.cur_size);

    Ok(old_addr)
  }

  fn free(&mut self, addr: u64) -> SwppRawResult<()> {
    let end_addr = self
      .memory
      .remove(&addr)
      .ok_or(SwppErrorKind::InvalidAddr(addr))?;

    self.cur_size -= end_addr.len() as u64;

    Ok(())
  }

  fn read(&self, addr: u64, size: AccessSize) -> SwppRawResult<u64> {
    let (target_start, target_mem) = self
      .memory
      .range((Bound::Included(HEAP_OFFSET), Bound::Included(addr)))
      .last()
      .ok_or(SwppErrorKind::InvalidAddr(addr))?;

    if (target_start + target_mem.len() as u64) < addr {
      return Err(SwppErrorKind::InvalidAddr(addr));
    }

    let idx = (addr - target_start) as usize;

    let mut byte_arr = [0; 8];
    for i in 0..size.into() {
      byte_arr[i] = target_mem
        .get(idx + i)
        .ok_or(SwppErrorKind::InvalidAddr(addr))?
        .to_owned();
    }
    Ok(u64::from_le_bytes(byte_arr))
  }

  fn write(&mut self, addr: u64, val: u64, size: AccessSize) -> SwppRawResult<()> {
    let (target_start, target_mem) = self
      .memory
      .range_mut((Bound::Included(HEAP_OFFSET), Bound::Included(addr)))
      .last()
      .ok_or(SwppErrorKind::InvalidAddr(addr))?;

    if (target_start + target_mem.len() as u64) < addr {
      return Err(SwppErrorKind::InvalidAddr(addr));
    }

    let idx = (addr - target_start) as usize;
    let val_bytes = val.to_le_bytes();

    for i in 0..size.into() {
      let target_byte = target_mem
        .get_mut(idx + i)
        .ok_or(SwppErrorKind::InvalidAddr(addr))?;
      *target_byte = val_bytes[i];
    }

    Ok(())
  }

  fn get_allocation_size(&self, addr: u64) -> Option<u64> {
    self.memory.get(&addr).map(|v| v.len() as u64)
  }

  fn get_allocation_bounds(&self, addr: u64) -> Option<(u64, u64)> {
    let (target_start, target_mem) = self
      .memory
      .range((Bound::Included(HEAP_OFFSET), Bound::Included(addr)))
      .last()?;

    if (*target_start + target_mem.len() as u64) <= addr {
      return None;
    }

    Some((*target_start, target_mem.len() as u64))
  }
}
