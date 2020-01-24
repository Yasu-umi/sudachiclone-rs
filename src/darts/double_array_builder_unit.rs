pub trait DoubleArrayBuilderUnit {
  fn set_has_leaf(&mut self, has_leaf: bool);
  fn set_value(&mut self, value: u32);
  fn set_label(&mut self, label: u8);
  fn set_offset(&mut self, offset: usize);
}

impl DoubleArrayBuilderUnit for u32 {
  fn set_has_leaf(&mut self, has_leaf: bool) {
    if has_leaf {
      *self |= 1 << 8;
    } else {
      *self &= !(1 << 8);
    }
  }
  fn set_value(&mut self, value: u32) {
    *self = value | (1 << 31);
  }
  fn set_label(&mut self, label: u8) {
    *self = (*self & !0xFF) | label as u32;
  }
  fn set_offset(&mut self, offset: usize) {
    if offset >= 1 << 29 {
      panic!("failed to modify unit: too large offset");
    }
    *self &= (1 << 31) | (1 << 8) | 0xFF;
    if offset < 1 << 21 {
      *self |= (offset << 10) as u32;
    } else {
      *self |= ((offset << 2) | (1 << 9)) as u32;
    }
  }
}
