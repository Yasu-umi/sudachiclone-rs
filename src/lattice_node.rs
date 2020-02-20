use std::fmt;
use std::sync::{Arc, Mutex};

use rand::Rng;

use super::dictionary_lib::lexicon_set::LexiconSet;
use super::dictionary_lib::word_info::WordInfo;

pub struct LatticeNode {
  pub id: u32,
  pub start: usize,
  pub end: usize,
  pub total_cost: i32,
  pub word_id: usize,
  _is_oov: bool,
  pub is_defined: bool,
  pub best_previous_node: Option<Arc<Mutex<LatticeNode>>>,
  pub is_connected_to_bos: bool,
  extra_word_info: Option<WordInfo>,
  lexicon: Option<Arc<Mutex<LexiconSet>>>,
  pub left_id: u32,
  pub right_id: u32,
  pub cost: i32,
}

impl LatticeNode {
  pub fn empty(left_id: u32, right_id: u32, cost: i32) -> LatticeNode {
    let id = rand::thread_rng().gen();
    let start = 0;
    let end = 0;
    let total_cost = 0;
    let _is_oov = false;
    let best_previous_node = None;
    let is_connected_to_bos = false;
    let extra_word_info = None;
    LatticeNode {
      id,
      start,
      end,
      total_cost,
      word_id: 0,
      _is_oov,
      is_defined: false,
      best_previous_node,
      is_connected_to_bos,
      extra_word_info,
      lexicon: None,
      left_id,
      right_id,
      cost,
    }
  }
  pub fn new(
    lexicon: Option<Arc<Mutex<LexiconSet>>>,
    left_id: u32,
    right_id: u32,
    cost: i32,
    word_id: usize,
  ) -> LatticeNode {
    let id = rand::thread_rng().gen();
    let start = 0;
    let end = 0;
    let total_cost = 0;
    let _is_oov = false;
    let best_previous_node = None;
    let is_connected_to_bos = false;
    let extra_word_info = None;
    LatticeNode {
      id,
      start,
      end,
      total_cost,
      word_id,
      _is_oov,
      is_defined: true,
      best_previous_node,
      is_connected_to_bos,
      extra_word_info,
      lexicon,
      left_id,
      right_id,
      cost,
    }
  }
  pub fn get_start(&self) -> usize {
    self.start
  }
  pub fn get_end(&self) -> usize {
    self.end
  }
  pub fn is_oov(&self) -> bool {
    self._is_oov
  }
  pub fn set_oov(&mut self) {
    self._is_oov = true;
  }
  pub fn get_path_cost(&self) -> i32 {
    self.cost
  }
  pub fn get_word_id(&self) -> usize {
    self.word_id
  }
  pub fn get_dictionary_id(&self) -> Option<usize> {
    if !self.is_defined || self.extra_word_info.is_some() {
      None
    } else {
      Some(
        self
          .lexicon
          .as_ref()
          .unwrap()
          .lock()
          .unwrap()
          .get_dictionary_id(self.word_id),
      ) // self.word_id >> 28
    }
  }
  pub fn get_word_info(&self) -> WordInfo {
    if !self.is_defined {
      return build_undefined_word_info();
    }
    match self.extra_word_info.clone() {
      Some(info) => info,
      None => self
        .lexicon
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .get_word_info(self.word_id),
    }
  }
  pub fn set_word_info(&mut self, word_info: WordInfo) {
    self.extra_word_info = Some(word_info);
    self.is_defined = true;
  }
  pub fn to_str(&self) -> String {
    let surface = if
    /* self.word_id >= 0 || */
    self.extra_word_info.is_none() {
      self.get_word_info().surface
    } else {
      String::from("(None)")
    };
    format!(
      "{} {} {}({}) {} {} {}",
      self.start, self.end, surface, self.word_id, self.left_id, self.right_id, self.cost,
    )
  }
}

const NULL_SURFACE: &str = "(null)";

fn build_undefined_word_info() -> WordInfo {
  WordInfo {
    surface: NULL_SURFACE.to_string(),
    head_word_length: 0,
    pos_id: -1,
    normalized_form: NULL_SURFACE.to_string(),
    dictionary_form_word_id: -1,
    dictionary_form: NULL_SURFACE.to_string(),
    reading_form: NULL_SURFACE.to_string(),
    a_unit_split: vec![],
    b_unit_split: vec![],
    word_structure: vec![],
  }
}

impl fmt::Debug for LatticeNode {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    writeln!(f, "{}", self.to_str())?;
    Ok(())
  }
}
impl PartialEq for LatticeNode {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}
