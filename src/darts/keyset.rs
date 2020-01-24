pub struct Keyset<'a> {
  keys: &'a [&'a [u8]],
  lengths: &'a [usize],
  values: &'a [u32],
}

impl<'a> Keyset<'a> {
  pub fn new(keys: &'a [&'a [u8]], lengths: &'a [usize], values: &'a [u32]) -> Keyset<'a> {
    Keyset {
      keys,
      lengths,
      values,
    }
  }
  pub fn has_values(&self) -> bool {
    !self.values.is_empty()
  }
  pub fn has_lengths(&self) -> bool {
    !self.lengths.is_empty()
  }
  pub fn get_key(&self, key_id: usize) -> &'a [u8] {
    self.keys[key_id]
  }
  pub fn get_char(&self, key_id: usize, char_id: usize) -> u8 {
    if self.has_lengths() && char_id >= self.lengths[key_id] {
      0
    } else {
      self.keys[key_id][char_id]
    }
  }
  pub fn num_keys(&self) -> usize {
    self.keys.len()
  }
  pub fn get_value(&self, id: usize) -> u32 {
    if self.has_values() {
      self.values[id]
    } else {
      id as u32
    }
  }
  pub fn get_length(&self, id: usize) -> usize {
    if self.has_lengths() {
      self.lengths[id]
    } else {
      let mut length: usize = 0;
      while self.keys[id][length] != 0 {
        length += 1;
      }
      length
    }
  }
}
