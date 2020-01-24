use std::io::Cursor;
use std::mem::size_of;

use byteorder::{LittleEndian, ReadBytesExt};

use super::double_array_builder::DoubleArrayBuilder;
use super::double_array_unit::DoubleArrayUnit;
use super::keyset::Keyset;

#[derive(Default)]
pub struct DoubleArrayTrie {
  array: Vec<u32>,
  size: usize,
}

impl DoubleArrayTrie {
  pub fn build(&mut self, keys: &[&[u8]], values: &[u32]) {
    let lengths: Vec<usize> = keys.iter().map(|k| k.len()).collect();
    let keyset = Keyset::new(keys, &lengths, values);
    let mut builder = DoubleArrayBuilder::new();
    builder.build(&keyset);
    let (size, buf) = builder.copy();
    self.size = size;
    self.array = buf;
  }
  pub fn size(&self) -> usize {
    self.size
  }
  fn unit_size(&self) -> usize {
    size_of::<u32>()
  }
  pub fn total_size(&self) -> usize {
    self.unit_size() * self.size()
  }
  pub fn get_array(&self) -> &Vec<u32> {
    &self.array
  }
  pub fn set_array(&mut self, array: &[u8], size: usize) {
    let mut buf = vec![0u32; size];
    Cursor::new(array)
      .read_u32_into::<LittleEndian>(&mut buf)
      .unwrap();
    self.array = buf;
    self.size = size;
  }
  pub fn common_prefix_search(&self, key: &[u8]) -> Vec<(i32, usize)> {
    let length = key.len() as u64;
    let max_num_results = length as u64;
    let mut num_results = 0;
    let mut node_pos: usize = 0;
    let mut unit = &self.array[node_pos];
    node_pos ^= unit.offset();
    let mut results = vec![];
    for i in 0..length {
      let i_usize = i as usize;
      node_pos ^= key[i_usize] as usize;
      unit = &self.array[node_pos];
      if unit.label() != key[i_usize] {
        return results;
      }
      node_pos ^= unit.offset();
      if unit.has_leaf() {
        if num_results < max_num_results {
          results.push((self.array[node_pos].value(), (i_usize + 1)));
        }
        num_results += 1;
      }
    }
    results
  }
}
