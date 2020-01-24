use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use super::oov_provider_plugin::{OovProviderPlugin, OovProviderPluginSetupErr};
use crate::dictionary_lib::grammar::GetPartOfSpeech;
use crate::dictionary_lib::grammar::Grammar;
use crate::dictionary_lib::word_info::WordInfo;
use crate::lattice_node::LatticeNode;
use crate::utf8_input_text::InputText;

#[derive(Debug)]
pub struct SimpleOovPlugin {
  left_id: u32,
  right_id: u32,
  cost: i32,
  oov_pos_strings: Vec<String>,
  oov_pos_id: i16,
}

impl SimpleOovPlugin {
  pub fn new(json_obj: &Value) -> SimpleOovPlugin {
    let left_id = get_u64_by_key(json_obj, "leftId") as u32;
    let right_id = get_u64_by_key(json_obj, "rightId") as u32;
    let cost = get_i64_by_key(json_obj, "cost") as i32;
    let oov_pos_strings = json_obj
      .get("oovPOS")
      .map(|i| i.as_array())
      .flatten()
      .unwrap()
      .iter()
      .map(|i| i.as_str().unwrap().to_string())
      .collect();
    SimpleOovPlugin {
      left_id,
      right_id,
      cost,
      oov_pos_strings,
      oov_pos_id: -1,
    }
  }
}

impl<T: InputText> OovProviderPlugin<T> for SimpleOovPlugin {
  fn setup(&mut self, grammar: Rc<RefCell<Grammar>>) -> Result<(), OovProviderPluginSetupErr> {
    let strings: Vec<&str> = self.oov_pos_strings.iter().map(|s| s.as_str()).collect();
    self.oov_pos_id = grammar
      .borrow()
      .get_part_of_speech_id(&strings)
      .map(|i| i as i16)
      .unwrap_or(-1);
    Ok(())
  }
  fn provide_oov(
    &self,
    input_text: &T,
    offset: usize,
    has_other_words: bool,
  ) -> Vec<Rc<RefCell<LatticeNode>>> {
    if !has_other_words {
      let mut node = LatticeNode::empty(self.left_id, self.right_id, self.cost);
      node.set_oov();
      let length = input_text.get_word_candidate_length(offset);
      let s = input_text.get_substring(offset, offset + length).unwrap();
      let info = WordInfo {
        surface: s.to_string(),
        head_word_length: length,
        pos_id: self.oov_pos_id,
        normalized_form: s.to_string(),
        dictionary_form_word_id: 1,
        dictionary_form: s.to_string(),
        reading_form: String::from(""),
        a_unit_split: vec![],
        b_unit_split: vec![],
        word_structure: vec![],
      };
      node.set_word_info(info);
      vec![Rc::new(RefCell::new(node))]
    } else {
      vec![]
    }
  }
}

fn get_u64_by_key(v: &Value, k: &str) -> i64 {
  v.get(k).map(|i| i.as_u64()).flatten().unwrap() as i64
}

fn get_i64_by_key(v: &Value, k: &str) -> u64 {
  v.get(k).map(|i| i.as_u64()).flatten().unwrap() as u64
}
