pub struct DoubleArrayBuilderExtraUnit {
  pub prev: usize,
  pub next: usize,
  pub is_fixed: bool,
  pub is_used: bool,
}

impl DoubleArrayBuilderExtraUnit {
  pub fn new() -> DoubleArrayBuilderExtraUnit {
    DoubleArrayBuilderExtraUnit {
      prev: 0,
      next: 0,
      is_fixed: false,
      is_used: false,
    }
  }
}
