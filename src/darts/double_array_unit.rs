pub trait DoubleArrayUnit {
  fn offset(&self) -> usize;
  fn label(&self) -> u8;
  fn has_leaf(&self) -> bool;
  fn value(&self) -> i32;
}

impl DoubleArrayUnit for u32 {
  fn offset(&self) -> usize {
    ((self >> 10) << ((self & (1 << 9)) >> 6)) as usize
  }
  fn label(&self) -> u8 {
    (self & ((1 << 31) | 0xFF)) as u8
  }
  fn has_leaf(&self) -> bool {
    ((self >> 8) & 1u32) == 1u32
  }
  fn value(&self) -> i32 {
    (self & ((1 << 31) - 1)) as i32
  }
}
