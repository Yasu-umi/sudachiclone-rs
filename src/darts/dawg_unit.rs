pub trait DawgUnit {
  fn unit(&self) -> usize;
  fn child(&self) -> usize;
  fn has_sibling(&self) -> bool;
  fn value(&self) -> u32;
  fn is_state(&self) -> bool;
}

impl DawgUnit for usize {
  fn unit(&self) -> usize {
    *self
  }
  fn child(&self) -> usize {
    self >> 2
  }
  fn has_sibling(&self) -> bool {
    self & 1 == 1
  }
  fn value(&self) -> u32 {
    (self >> 1) as u32
  }
  fn is_state(&self) -> bool {
    (self & 2) == 2
  }
}
