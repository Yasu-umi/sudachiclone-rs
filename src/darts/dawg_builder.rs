use std::cmp::Ordering;
use std::num::Wrapping;

// use succinct::{BitRankSupport, BitVec, BitVecMut, BitVecPush, BitVector, JacobsonRank};

use super::dawg_node::DawgNode;
use super::dawg_unit::DawgUnit;

const INITIAL_TABLE_SIZE: usize = 1 << 10;

pub struct DawgBuilder {
  nodes: Vec<DawgNode>,
  units: Vec<usize>,
  labels: Vec<u8>,
  // _is_intersections: BitVector,
  // is_intersections: JacobsonRank<BitVector>,
  _is_intersections: Vec<bool>,
  is_intersections: Vec<bool>,
  table: Vec<usize>,
  node_stack: Vec<usize>,
  recycle_bin: Vec<usize>,
  num_states: usize,
}

// https://gist.github.com/badboy/6267743#32-bit-mix-functions
fn hash(key: u32) -> u32 {
  let mut key = Wrapping(key);
  key = !key + (key << 15); // key = (key << 15) - key - 1;
  key ^= key >> 12;
  key += key << 2;
  key ^= key >> 4;
  key *= Wrapping(2057); // key = (key + (key << 3)) + (key << 11);
  key ^= key >> 16;
  key.0
}

impl DawgBuilder {
  pub fn new() -> DawgBuilder {
    DawgBuilder {
      nodes: vec![],
      units: vec![],
      labels: vec![],
      // _is_intersections: BitVector::new(),
      // is_intersections: JacobsonRank::new(BitVector::with_fill(10, false)),
      _is_intersections: Vec::new(),
      is_intersections: Vec::new(),
      table: vec![],
      node_stack: vec![],
      recycle_bin: vec![],
      num_states: 0,
    }
  }
  pub fn root(&self) -> usize {
    0
  }
  pub fn child(&self, id: usize) -> usize {
    self.units[id].child()
  }
  pub fn sibling(&self, id: usize) -> usize {
    if self.units[id].has_sibling() {
      id + 1
    } else {
      0
    }
  }
  pub fn value(&self, id: usize) -> u32 {
    self.units[id].value()
  }
  pub fn is_leaf(&self, id: usize) -> bool {
    self.label(id) != 0
  }
  pub fn label(&self, id: usize) -> u8 {
    self.labels[id]
  }
  pub fn is_intersection(&self, id: usize) -> bool {
    // self.is_intersections.get_bit(id as u64)
    *self.is_intersections.get(id).unwrap()
  }
  pub fn intersection_id(&self, id: usize) -> usize {
    // (self.is_intersections.rank1(id as u64) - 1) as usize
    self.is_intersections[0..id].iter().filter(|x| **x).count()
  }
  pub fn num_intersections(&self) -> usize {
    // too slow?
    // self.is_intersections.inner().iter().filter(|x| *x).count()
    self.is_intersections.iter().filter(|x| **x).count()
  }
  pub fn size(&self) -> usize {
    self.units.len()
  }
  pub fn init(&mut self) {
    self.table = vec![0; INITIAL_TABLE_SIZE];
    self.append_node();
    self.append_unit();
    self.num_states = 1;
    self.nodes[0].label = 0xFF;
    self.node_stack.push(0);
  }
  pub fn finish(&mut self) {
    self.flush(0);

    self.units[0] = self.nodes[0].unit();
    self.labels[0] = self.nodes[0].label;

    self.nodes.clear();
    self.table.clear();
    self.node_stack.clear();
    self.recycle_bin.clear();

    // self.is_intersections = JacobsonRank::new(self._is_intersections.clone());
    self.is_intersections = self._is_intersections.clone();
    self._is_intersections.clear();
  }
  pub fn insert(&mut self, key: &[u8], length: usize, value: u32) {
    let mut id = 0;
    let mut key_pos = 0;
    while key_pos <= length {
      let child_id = self.nodes[id].child;
      if child_id == 0 {
        break;
      }
      let key_label = key[key_pos as usize];
      if key_pos < length && key_label == 0 {
        panic!("failed to insert key: invalid null character");
      }
      let unit_label = self.nodes[child_id].label;
      match key_label.cmp(&unit_label) {
        Ordering::Greater => {
          self.nodes[child_id].has_sibling = true;
          self.flush(child_id);
          break;
        }
        Ordering::Less => panic!("failed to insert key: wrong key order"),
        Ordering::Equal => {}
      }
      id = child_id;
      key_pos += 1;
    }
    if key_pos > length {
      return;
    }
    while key_pos <= length {
      let key_label = if key_pos < length { key[key_pos] } else { 0 };
      let child_id = self.append_node();
      if self.nodes[id].child == 0 {
        self.nodes[child_id].is_state = true;
      }
      self.nodes[child_id].sibling = self.nodes[id].child;
      self.nodes[child_id].label = key_label;
      self.nodes[id].child = child_id;
      self.node_stack.push(child_id);
      id = child_id;
      key_pos += 1;
    }
    self.nodes[id].child = value as usize;
  }
  fn append_unit(&mut self) -> usize {
    // self._is_intersections.push_bit(false);
    self._is_intersections.push(false);
    self.units.push(0);
    self.labels.push(0);
    // self._is_intersections.bit_len() as usize - 1
    self._is_intersections.len() - 1
  }
  fn append_node(&mut self) -> usize {
    if self.recycle_bin.is_empty() {
      let id = self.nodes.len();
      self.nodes.push(DawgNode::new());
      id
    } else {
      let id = self.recycle_bin.pop().unwrap();
      self.nodes[id] = DawgNode::new();
      id
    }
  }
  fn flush(&mut self, id: usize) {
    while self.node_stack.last().unwrap() != &id {
      let node_id = self.node_stack.pop().unwrap();
      if self.num_states >= self.table.len() - (self.table.len() >> 2) {
        self.expand_table();
      }
      let mut num_siblings = 0;
      let mut i = node_id;
      while i != 0 {
        num_siblings += 1;
        i = self.nodes[i].sibling;
      }

      let (hash_id, mut match_id) = self.find_node(node_id);
      if match_id != 0 {
        // self._is_intersections.set_bit(match_id as u64, true);
        self._is_intersections[match_id] = true;
      } else {
        let mut unit_id = 0;
        for _ in 0..num_siblings {
          unit_id = self.append_unit();
        }
        let mut i = node_id;
        while i != 0 {
          self.units[unit_id] = self.nodes[i].unit();
          self.labels[unit_id] = self.nodes[i].label;
          unit_id -= 1;
          i = self.nodes[i].sibling;
        }
        let _match_id = unit_id + 1;
        match_id = _match_id;
        self.table[hash_id] = _match_id;
        self.num_states += 1;
      }

      let mut i = node_id;
      while i != 0 {
        let next = self.nodes[i].sibling;
        self.free_node(i);
        i = next;
      }

      self.nodes[*self.node_stack.last().unwrap()].child = match_id;
    }
    self.node_stack.pop();
  }
  fn expand_table(&mut self) {
    let table_size = self.table.len() << 1;
    self.table.clear();
    self.table = vec![0; table_size];
    for i in 1..self.units.len() {
      if self.labels[i] == 0 || self.units[i].is_state() {
        let hash_id = self.find_unit(i);
        self.table[hash_id] = i;
      }
    }
  }
  fn find_unit(&self, id: usize) -> usize {
    let mut hash_id = self.hash_unit(id) % self.table.len();
    loop {
      let unit_id = self.table[hash_id];
      if unit_id == 0 {
        break;
      }
      hash_id = (hash_id + 1) % self.table.len();
    }
    hash_id
  }
  fn find_node(&self, node_id: usize) -> (usize, usize) {
    let mut hash_id = self.hash_node(node_id) % self.table.len();
    loop {
      let unit_id = self.table[hash_id];
      if unit_id == 0 {
        break;
      }
      if self.are_equal(node_id, unit_id) {
        return (hash_id, unit_id);
      }
      hash_id = (hash_id + 1) % self.table.len();
    }
    (hash_id, 0)
  }
  fn are_equal(&self, node_id: usize, unit_id: usize) -> bool {
    let mut unit_id = unit_id;
    let mut i = self.nodes[node_id].sibling;
    while i != 0 {
      if !self.units[unit_id].has_sibling() {
        return false;
      }
      unit_id += 1;
      i = self.nodes[i].sibling;
    }
    if self.units[unit_id].has_sibling() {
      return false;
    }
    let mut i = node_id;
    while i != 0 {
      if self.nodes[i].unit() != self.units[unit_id].unit()
        || self.nodes[i].label != self.labels[unit_id]
      {
        return false;
      }
      i = self.nodes[i].sibling;
      unit_id -= 1;
    }
    true
  }
  fn hash_unit(&self, id: usize) -> usize {
    let mut hash_value = Wrapping(0);
    let mut id = id;
    while id != 0 {
      let unit = self.units[id].unit();
      let label = self.labels[id];
      hash_value ^= Wrapping(hash((((label as usize) << 24) ^ unit) as u32));
      if !self.units[id].has_sibling() {
        break;
      }
      id += 1;
    }
    hash_value.0 as usize
  }
  fn hash_node(&self, id: usize) -> usize {
    let mut hash_value = Wrapping(0);
    let mut id = id;
    while id != 0 {
      let unit = self.nodes[id].unit();
      let label = self.nodes[id].label as usize;
      hash_value ^= Wrapping(hash(((label << 24) ^ unit) as u32));
      id = self.nodes[id].sibling;
    }
    hash_value.0 as usize
  }
  fn free_node(&mut self, id: usize) {
    self.recycle_bin.push(id);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_hash() {
    assert_eq!(3_399_731_875, hash(0));
    assert_eq!(316_017_654, hash(1));
  }
}
