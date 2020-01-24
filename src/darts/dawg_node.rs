pub struct DawgNode {
  pub child: usize,
  pub sibling: usize,
  pub label: u8,
  pub is_state: bool,
  pub has_sibling: bool,
}

impl DawgNode {
  pub fn new() -> DawgNode {
    DawgNode {
      child: 0,
      sibling: 0,
      label: 0,
      is_state: false,
      has_sibling: false,
    }
  }
  pub fn unit(&self) -> usize {
    if self.label == 0 {
      (self.child << 1) | (if self.has_sibling { 1 } else { 0 })
    } else {
      (self.child << 2)
        | (if self.is_state { 2 } else { 0 })
        | (if self.has_sibling { 1 } else { 0 })
    }
  }
}
