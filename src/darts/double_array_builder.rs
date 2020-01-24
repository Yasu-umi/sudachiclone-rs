use super::dawg_builder::DawgBuilder;
use super::double_array_builder_extra_unit::DoubleArrayBuilderExtraUnit;
use super::double_array_builder_unit::DoubleArrayBuilderUnit;
use super::keyset::Keyset;

const BLOCK_SIZE: usize = 256;
const NUM_EXTRA_BLOCKS: usize = 16;
const NUM_EXTRAS: usize = BLOCK_SIZE * NUM_EXTRA_BLOCKS;
const LOWER_MASK: usize = 0xFF;
const UPPER_MASK: usize = 0xFF << 21;

pub struct DoubleArrayBuilder {
  labels: Vec<u8>,
  units: Vec<u32>,
  extras: Vec<DoubleArrayBuilderExtraUnit>,
  table: Vec<usize>,
  extras_head: usize,
}

impl DoubleArrayBuilder {
  pub fn new() -> DoubleArrayBuilder {
    DoubleArrayBuilder {
      labels: vec![],
      units: vec![],
      extras: vec![],
      table: vec![],
      extras_head: 0,
    }
  }
  pub fn build(&mut self, keyset: &Keyset) {
    if keyset.has_values() {
      let mut dawg_builder = DoubleArrayBuilder::build_dawg(keyset);
      self.build_from_dawg(&mut dawg_builder);
    } else {
      self.build_from_keyset(keyset);
    }
  }
  pub fn copy(self) -> (usize, Vec<u32>) {
    (self.units.len(), self.units)
  }
  fn build_dawg(keyset: &Keyset) -> DawgBuilder {
    let mut dawg_builder = DawgBuilder::new();
    dawg_builder.init();
    for i in 0..keyset.num_keys() {
      dawg_builder.insert(keyset.get_key(i), keyset.get_length(i), keyset.get_value(i));
    }
    dawg_builder.finish();
    dawg_builder
  }
  fn build_from_dawg(&mut self, dawg_builder: &mut DawgBuilder) {
    let mut num_units = 1;
    while num_units < dawg_builder.size() {
      num_units <<= 1;
    }
    self.units.reserve(num_units);
    self.table = vec![0; dawg_builder.num_intersections()];
    self.extras = (0..NUM_EXTRAS)
      .map(|_| DoubleArrayBuilderExtraUnit::new())
      .collect();

    self.reserve_id(0);
    self.extras[0].is_used = true;
    self.units[0].set_offset(1);
    self.units[0].set_label(0);

    if dawg_builder.child(dawg_builder.root()) != 0 {
      self._build_from_dawg(dawg_builder, dawg_builder.root(), 0);
    }

    self.fix_all_blocks();

    self.extras.clear();
    self.labels.clear();
    self.table.clear();
  }
  fn _build_from_dawg(&mut self, dawg_builder: &mut DawgBuilder, dawg_id: usize, dict_id: usize) {
    let mut dawg_builder_child_id = dawg_builder.child(dawg_id);
    if dawg_builder.is_intersection(dawg_builder_child_id) {
      let intersection_id = dawg_builder.intersection_id(dawg_builder_child_id);
      let mut offset = self.table[intersection_id];
      if offset != 0 {
        offset ^= dict_id;
        if offset & UPPER_MASK == 0 || offset & LOWER_MASK == 0 {
          if dawg_builder.is_leaf(dawg_builder_child_id) {
            self.units[dict_id].set_has_leaf(true);
          }
          self.units[dict_id].set_offset(offset);
          return;
        }
      }
    }
    let offset = self.arrange_from_dawg_builder(dawg_builder, dawg_id, dict_id);
    if dawg_builder.is_intersection(dawg_builder_child_id) {
      self.table[dawg_builder.intersection_id(dawg_builder_child_id)] = offset;
    }
    while {
      let child_label = dawg_builder.label(dawg_builder_child_id);
      let dict_child_id = offset ^ child_label as usize;
      if child_label != 0 {
        self._build_from_dawg(dawg_builder, dawg_builder_child_id, dict_child_id);
      }
      dawg_builder_child_id = dawg_builder.sibling(dawg_builder_child_id);
      dawg_builder_child_id != 0
    } {}
  }
  fn arrange_from_dawg_builder(
    &mut self,
    dawg_builder: &mut DawgBuilder,
    dawg_id: usize,
    dict_id: usize,
  ) -> usize {
    self.labels.clear();

    let mut dawg_child_id = dawg_builder.child(dawg_id);
    while dawg_child_id != 0 {
      self.labels.push(dawg_builder.label(dawg_child_id));
      dawg_child_id = dawg_builder.sibling(dawg_child_id);
    }

    let offset = self.find_valid_offset(dict_id);
    self.units[dict_id].set_offset(dict_id ^ offset);

    dawg_child_id = dawg_builder.child(dawg_id);
    for i in 0..self.labels.len() {
      let dict_child_id = offset ^ self.labels[i] as usize;
      self.reserve_id(dict_child_id);

      if dawg_builder.is_leaf(dawg_child_id) {
        self.units[dict_id].set_has_leaf(true);
        self.units[dict_child_id].set_value(dawg_builder.value(dawg_child_id));
      } else {
        self.units[dict_child_id].set_label(self.labels[i]);
      }
      dawg_child_id = dawg_builder.sibling(dawg_child_id);
    }
    self.get_extra(offset).is_used = true;

    offset
  }
  fn build_from_keyset(&mut self, keyset: &Keyset) {
    let mut num_units = 1;
    while num_units < keyset.num_keys() {
      num_units <<= 1;
    }
    self.units.reserve(num_units);
    // self.extras.reset();
    self.reserve_id(0);
    self.extras[0].is_used = true;
    self.units[0].set_offset(1);
    self.units[0].set_label(0);

    if keyset.num_keys() > 0 {
      self._build_from_keyset(keyset, 0, keyset.num_keys(), 0, 0);
    }
    self.fix_all_blocks();
    self.extras.clear();
    self.labels.clear();
  }
  fn _build_from_keyset(
    &mut self,
    keyset: &Keyset,
    start: usize,
    end: usize,
    depth: usize,
    dict_id: usize,
  ) {
    let mut start = start;
    let offset = self.arrange_from_keyset(keyset, start, end, depth, dict_id);

    while start < end {
      if keyset.get_char(start, depth) == 0 {
        break;
      }
      start += 1;
    }
    if start == end {
      return;
    }

    let mut last_start = start;
    let mut last_label = keyset.get_char(start, depth);
    start += 1;
    while start <= end {
      let label = keyset.get_char(start, depth);
      if label != last_label {
        self._build_from_keyset(
          keyset,
          last_start,
          start,
          depth + 1,
          offset ^ last_label as usize,
        );
        last_start = start;
        last_label = keyset.get_char(start, depth);
      }
      start += 1;
    }
    self._build_from_keyset(
      keyset,
      last_start,
      start,
      depth + 1,
      offset ^ last_label as usize,
    );
  }
  fn arrange_from_keyset(
    &mut self,
    keyset: &Keyset,
    start: usize,
    end: usize,
    depth: usize,
    dict_id: usize,
  ) -> usize {
    self.labels = vec![];
    let mut value: Option<u32> = None;
    for i in start..end {
      let label = keyset.get_char(i, depth);
      if label == 0 && keyset.has_lengths() && depth < keyset.get_length(i) {
        panic!("failed to build double-array: invalid null character");
      }
      /*
      if label == 0 && keyset.get_value(i) < 0 {
        panic!("failed to build double-array: negative value");
      }
      */
      if value.is_none() {
        value = Some(keyset.get_value(i));
      }
      if self.labels.is_empty() {
        self.labels.push(label);
      } else if label != self.labels[self.labels.len() - 1] {
        if label < self.labels[self.labels.len() - 1] {
          panic!("failed to build double-array: wrong key order");
        }
        self.labels.push(label);
      }
    }
    let offset = self.find_valid_offset(dict_id);
    self.units[dict_id].set_offset(dict_id ^ offset);
    for i in 0..self.labels.len() {
      let dict_child_id = offset ^ self.labels[i] as usize;
      self.reserve_id(dict_child_id);
      if self.labels[i] == 0 {
        self.units[dict_id].set_has_leaf(true);
        self.units[dict_child_id].set_value(value.unwrap());
      } else {
        self.units[dict_child_id].set_label(self.labels[i]);
      }
    }
    self.get_extra(offset).is_used = true;
    offset
  }
  fn find_valid_offset(&mut self, id: usize) -> usize {
    if self.extras_head >= self.units.len() {
      return self.units.len() | (id & LOWER_MASK);
    }
    let mut unfixed_id = self.extras_head;
    while {
      let offset = unfixed_id ^ self.labels[0] as usize;
      if self.is_valid_offset(id, offset) {
        return offset;
      }
      unfixed_id = self.get_extra(unfixed_id).next;
      unfixed_id != self.extras_head
    } {}
    self.units.len() | (id & LOWER_MASK)
  }
  fn is_valid_offset(&mut self, id: usize, offset: usize) -> bool {
    if self.get_extra(offset).is_used {
      return false;
    }
    let rel_offset = id ^ offset;
    if rel_offset ^ LOWER_MASK > 0 && rel_offset & UPPER_MASK > 0 {
      return false;
    }
    for i in 1..self.labels.len() {
      if self.get_extra(offset ^ self.labels[i] as usize).is_fixed {
        return false;
      }
    }
    true
  }
  fn reserve_id(&mut self, id: usize) {
    if id >= self.units.len() {
      self.expand_units();
    }
    if id == self.extras_head {
      self.extras_head = self.get_extra(id).next;
      if self.extras_head == id {
        self.extras_head = self.units.len();
      }
    }
    let prev = self.get_extra(id).prev;
    let next = self.get_extra(id).next;
    self.get_extra(prev).next = next;
    self.get_extra(next).prev = prev;
  }
  fn expand_units(&mut self) {
    let src_num_units = self.units.len();
    let src_num_blocks = self.num_blocks();
    let dest_num_units = src_num_units + BLOCK_SIZE;
    let dest_num_blocks = src_num_blocks + 1;

    if dest_num_blocks > NUM_EXTRA_BLOCKS {
      self.fix_block(src_num_blocks - NUM_EXTRA_BLOCKS);
    }
    self.units.resize_with(dest_num_units, Default::default);

    if dest_num_blocks > NUM_EXTRA_BLOCKS {
      for id in src_num_units..dest_num_blocks {
        let extra = self.get_extra(id);
        extra.is_used = false;
        extra.is_fixed = false;
      }
    }

    for id in src_num_units + 1..dest_num_units {
      self.get_extra(id - 1).next = id;
      self.get_extra(id).prev = id - 1;
    }
    self.get_extra(src_num_units).prev = dest_num_units - 1;
    self.get_extra(dest_num_units - 1).next = src_num_units;

    self.get_extra(src_num_units).prev = self.get_extra(self.extras_head).prev;
    self.get_extra(dest_num_units - 1).next = self.extras_head;

    let head_prev = self.get_extra(self.extras_head).prev;
    self.get_extra(head_prev).next = src_num_units;
    self.get_extra(self.extras_head).prev = dest_num_units - 1;
  }
  fn fix_all_blocks(&mut self) {
    let start = if self.num_blocks() > NUM_EXTRA_BLOCKS {
      self.num_blocks() - NUM_EXTRA_BLOCKS
    } else {
      0
    };
    let end = self.num_blocks();
    for block_id in start..end {
      self.fix_block(block_id);
    }
  }
  fn fix_block(&mut self, block_id: usize) {
    let start = block_id * BLOCK_SIZE;
    let end = start * BLOCK_SIZE;
    let mut unused_offset = 0;
    for offset in start..end {
      if !self.get_extra(offset).is_used {
        unused_offset = offset;
        break;
      }
    }
    for id in start..end {
      if !self.get_extra(id).is_fixed {
        self.reserve_id(id);
        self.units[id].set_label((id ^ unused_offset) as u8);
      }
    }
  }
  fn get_extra(&mut self, id: usize) -> &mut DoubleArrayBuilderExtraUnit {
    &mut self.extras[id % NUM_EXTRAS]
  }
  fn num_blocks(&self) -> usize {
    self.units.len() / BLOCK_SIZE
  }
}
