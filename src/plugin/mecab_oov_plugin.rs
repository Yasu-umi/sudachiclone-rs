use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IOError};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use thiserror::Error;

use super::oov_provider_plugin::ProvideOov;
use crate::dictionary_lib::category_type::CategoryType;
use crate::dictionary_lib::grammar::{GetPartOfSpeech, Grammar};
use crate::dictionary_lib::word_info::WordInfo;
use crate::lattice_node::LatticeNode;
use crate::utf8_input_text::InputText;

#[derive(Debug)]
struct CategoryInfo {
  is_invoke: bool,
  is_group: bool,
  length: usize,
}

#[derive(Debug)]
struct Oov {
  left_id: u32,
  right_id: u32,
  cost: i32,
  pos_id: Option<usize>,
}

impl Oov {
  fn new(left_id: u32, right_id: u32, cost: i32, pos_id: Option<usize>) -> Oov {
    Oov {
      left_id,
      right_id,
      cost,
      pos_id,
    }
  }
}

type Categories = HashMap<CategoryType, CategoryInfo>;
type OovsList = HashMap<CategoryType, Vec<Oov>>;

#[derive(Debug)]
pub struct MecabOovPlugin {
  categories: Categories,
  oovs_list: OovsList,
}

#[derive(Debug, Error)]
pub enum MecabOovPluginSetupErr {
  #[error("charDef is not defined")]
  CharDefNotDefinedErr,
  #[error("unkDef is not defined")]
  UnkDefNotDefinedErr,
  #[error("{0}")]
  IOError(#[from] IOError),
  #[error("{0}")]
  ParseIntError(#[from] ParseIntError),
  #[error("invalid format at line {0} in char.def")]
  InvalidCharFormatErr(usize),
  #[error("invalid format at line {0} in unk.def")]
  InvalidUnkFormatErr(usize),
  #[error("`{1}` is invalid type at line {0}")]
  InvalidTypeErr(usize, String),
  #[error("`{1}` is already defined at line {0}")]
  AlreadyDefinedErr(usize, String),
  #[error("`{1}` is not defined at line {0}")]
  NotDefinedErr(usize, String),
}

impl MecabOovPlugin {
  pub fn setup<P: AsRef<Path>>(
    resource_dir: P,
    json_obj: &Value,
    grammar: Arc<Mutex<Grammar>>,
  ) -> Result<MecabOovPlugin, MecabOovPluginSetupErr> {
    let resource_dir = resource_dir.as_ref();
    let chardef_path = json_obj
      .get("charDef")
      .map(|i| i.as_str())
      .flatten()
      .map(|i| resource_dir.join(i));
    let unkdef_path = json_obj
      .get("unkDef")
      .map(|i| i.as_str())
      .flatten()
      .map(|i| resource_dir.join(i));
    let categories = MecabOovPlugin::read_character_property(chardef_path)?;
    let oovs_list = MecabOovPlugin::read_oov(unkdef_path, &categories, grammar)?;
    Ok(MecabOovPlugin {
      categories,
      oovs_list,
    })
  }

  fn read_character_property_from_reader<R: BufRead>(
    reader: &mut R,
  ) -> Result<Categories, MecabOovPluginSetupErr> {
    let mut categories = HashMap::new();
    for (i, line) in reader.lines().enumerate() {
      let i = i + 1;
      let line = line?;
      let line = line.trim();
      if line.is_empty() || line.starts_with('#') || line.starts_with("0x") {
        continue;
      }
      let cols: Vec<&str> = line.split_whitespace().collect();
      if cols.len() < 4 {
        return Err(MecabOovPluginSetupErr::InvalidCharFormatErr(i));
      }
      if let Ok(_type) = CategoryType::from_str(cols[0]) {
        if categories.contains_key(&_type) {
          return Err(MecabOovPluginSetupErr::AlreadyDefinedErr(
            i,
            cols[0].to_string(),
          ));
        }
        let info = CategoryInfo {
          is_invoke: cols[1] != "0",
          is_group: cols[2] != "0",
          length: usize::from_str(cols[3])?,
        };
        categories.insert(_type, info);
      } else {
        return Err(MecabOovPluginSetupErr::InvalidTypeErr(
          i,
          cols[0].to_string(),
        ));
      }
    }
    Ok(categories)
  }

  fn read_character_property(
    chardef_path: Option<PathBuf>,
  ) -> Result<Categories, MecabOovPluginSetupErr> {
    if let Some(chardef_path) = chardef_path {
      let mut reader = BufReader::new(File::open(chardef_path)?);
      MecabOovPlugin::read_character_property_from_reader(&mut reader)
    } else {
      Err(MecabOovPluginSetupErr::CharDefNotDefinedErr)
    }
  }

  fn read_oov_from_reader<R: BufRead>(
    reader: &mut R,
    categories: &Categories,
    grammar: Arc<Mutex<Grammar>>,
  ) -> Result<OovsList, MecabOovPluginSetupErr> {
    let mut oovs_list: OovsList = HashMap::new();
    let grammar = grammar.lock().unwrap();
    for (i, line) in reader.lines().enumerate() {
      let i = i + 1;
      let line = line?;
      let line = line.trim();
      if !line.is_empty() {
        continue;
      }
      let cols: Vec<&str> = line.split(',').collect();
      if cols.len() < 10 {
        return Err(MecabOovPluginSetupErr::InvalidUnkFormatErr(i));
      }

      if let Ok(_type) = CategoryType::from_str(cols[0]) {
        if !categories.contains_key(&_type) {
          return Err(MecabOovPluginSetupErr::NotDefinedErr(
            i,
            cols[0].to_string(),
          ));
        }
        let oov = Oov::new(
          u32::from_str(cols[1])?,
          u32::from_str(cols[2])?,
          i32::from_str(cols[3])?,
          grammar.get_part_of_speech_id(&cols[4..10]),
        );
        if let Some(oovs) = oovs_list.get_mut(&_type) {
          oovs.push(oov);
        } else {
          oovs_list.insert(_type, vec![oov]);
        }
      } else {
        return Err(MecabOovPluginSetupErr::InvalidTypeErr(
          i,
          cols[0].to_string(),
        ));
      }
    }
    Ok(oovs_list)
  }

  fn read_oov(
    unkdef_path: Option<PathBuf>,
    categories: &Categories,
    grammar: Arc<Mutex<Grammar>>,
  ) -> Result<OovsList, MecabOovPluginSetupErr> {
    if let Some(unkdef_path) = unkdef_path {
      let mut reader = BufReader::new(File::open(unkdef_path)?);
      MecabOovPlugin::read_oov_from_reader(&mut reader, categories, grammar)
    } else {
      Err(MecabOovPluginSetupErr::UnkDefNotDefinedErr)
    }
  }

  fn get_oov_node(&self, text: &str, oov: &Oov, len: usize) -> Arc<Mutex<LatticeNode>> {
    let mut node = LatticeNode::empty(oov.left_id, oov.right_id, oov.cost);
    node.set_oov();
    let info = WordInfo {
      surface: text.to_string(),
      head_word_length: len,
      pos_id: oov.pos_id.map(|i| i as i16).unwrap_or(-1),
      normalized_form: text.to_string(),
      dictionary_form_word_id: -1,
      dictionary_form: text.to_string(),
      reading_form: String::from(""),
      a_unit_split: vec![],
      b_unit_split: vec![],
      word_structure: vec![],
    };
    node.set_word_info(info);
    Arc::new(Mutex::new(node))
  }
}

impl<T: InputText> ProvideOov<T> for &MecabOovPlugin {
  fn provide_oov(
    &self,
    input_text: &T,
    offset: usize,
    has_other_words: bool,
  ) -> Vec<Arc<Mutex<LatticeNode>>> {
    let len = input_text.get_char_category_continuous_length(offset);
    let mut nodes = vec![];
    if len < 1 {
      return nodes;
    }
    for category_type in input_text.get_char_category_types(offset, None) {
      if let Some(category_info) = self.categories.get(&category_type) {
        let mut l_len = len;
        if !category_info.is_invoke && has_other_words {
          continue;
        }
        let empty = vec![];
        let oovs = self.oovs_list.get(&category_type).unwrap_or(&empty);
        if category_info.is_group {
          let s = input_text.get_substring(offset, offset + len).unwrap();
          for oov in oovs {
            nodes.push(self.get_oov_node(&s, oov, len));
            l_len -= 1;
          }
        }
        for i in 1..=category_info.length {
          let sub_len = input_text.get_code_points_offset_length(offset, i);
          if sub_len > l_len {
            break;
          }
          let s = input_text.get_substring(offset, offset + sub_len).unwrap();
          for oov in oovs {
            nodes.push(self.get_oov_node(&s, oov, sub_len));
          }
        }
      }
    }
    nodes
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::borrow::Cow;
  use std::cmp::min;
  use std::collections::HashSet;

  struct MockInputText {
    text: String,
    types: Vec<HashSet<CategoryType>>,
  }
  impl MockInputText {
    fn set_category_type(&mut self, start: usize, end: usize, _type: CategoryType) {
      for i in start..end {
        self.types[i].insert(_type);
      }
    }
  }
  impl InputText for MockInputText {
    fn get_char_category_continuous_length(&self, index: usize) -> usize {
      let continuous_category = self.types[index].clone();
      for i in index + 1..self.text.len() {
        let continuous_category: Vec<_> =
          continuous_category.intersection(&self.types[i]).collect();
        if continuous_category.is_empty() {
          return i - index;
        }
      }
      self.text.len() - index
    }
    fn get_char_category_types(&self, start: usize, end: Option<usize>) -> HashSet<CategoryType> {
      if let Some(end) = end {
        let mut continuous_category = self.types[start].clone();
        for i in start + 1..end {
          continuous_category = continuous_category
            .intersection(&self.types[i])
            .cloned()
            .collect();
        }
        continuous_category
      } else {
        self.types[start].clone()
      }
    }
    fn get_substring(&self, start: usize, end: usize) -> Result<Cow<str>, ()> {
      Ok(
        self
          .text
          .chars()
          .take(min(end, self.text.len()))
          .skip(start)
          .collect(),
      )
    }
    fn get_code_points_offset_length(&self, _index: usize, code_point_offset: usize) -> usize {
      code_point_offset
    }
    fn get_word_candidate_length(&self, _index: usize) -> usize {
      0
    }
  }

  fn build_plugin() -> MecabOovPlugin {
    let mut plugin = MecabOovPlugin {
      categories: HashMap::new(),
      oovs_list: HashMap::new(),
    };
    plugin
      .oovs_list
      .insert(CategoryType::KANJI, vec![Oov::new(0, 0, 0, Some(1))]);
    plugin.oovs_list.insert(
      CategoryType::KANJINUMERIC,
      vec![Oov::new(0, 0, 0, Some(1)), Oov::new(0, 0, 0, Some(2))],
    );
    plugin
  }

  fn build_mocked_input_text() -> MockInputText {
    let mut mocked_input_text = MockInputText {
      text: String::from("あいうえお"),
      types: vec![HashSet::new(); 5],
    };
    mocked_input_text.set_category_type(0, 3, CategoryType::KANJI);
    mocked_input_text
  }

  #[test]
  fn test_provide_oov000() {
    let mut plugin = build_plugin();
    let category_info = CategoryInfo {
      is_group: false,
      is_invoke: false,
      length: 0,
    };
    plugin.categories.insert(CategoryType::KANJI, category_info);
    let mocked_input_text = build_mocked_input_text();

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, false);
    assert_eq!(0, nodes.len());
    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, true);
    assert_eq!(0, nodes.len());
  }

  #[test]
  fn test_provide_oov100() {
    let mut plugin = build_plugin();
    let category_info = CategoryInfo {
      is_group: false,
      is_invoke: true,
      length: 0,
    };
    plugin.categories.insert(CategoryType::KANJI, category_info);
    let mocked_input_text = build_mocked_input_text();

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, false);
    assert_eq!(0, nodes.len());
    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, true);
    assert_eq!(0, nodes.len());
  }

  #[test]
  fn test_provide_oov010() {
    let mut plugin = build_plugin();
    let category_info = CategoryInfo {
      is_group: true,
      is_invoke: false,
      length: 0,
    };
    plugin.categories.insert(CategoryType::KANJI, category_info);
    let mocked_input_text = build_mocked_input_text();

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, false);
    assert_eq!(1, nodes.len());

    let node = nodes[0].lock().unwrap();
    assert_eq!("あいう", node.get_word_info().surface);
    assert_eq!(3, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, true);
    assert_eq!(0, nodes.len());
  }

  #[test]
  fn test_provide_oov110() {
    let mut plugin = build_plugin();
    let category_info = CategoryInfo {
      is_group: true,
      is_invoke: true,
      length: 0,
    };
    plugin.categories.insert(CategoryType::KANJI, category_info);
    let mocked_input_text = build_mocked_input_text();

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, false);
    assert_eq!(1, nodes.len());

    let node = nodes[0].lock().unwrap();
    assert_eq!("あいう", node.get_word_info().surface);
    assert_eq!(3, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, true);
    assert_eq!(1, nodes.len());
  }

  #[test]
  fn test_provide_oov002() {
    let mut plugin = build_plugin();
    let category_info = CategoryInfo {
      is_group: false,
      is_invoke: false,
      length: 2,
    };
    plugin.categories.insert(CategoryType::KANJI, category_info);
    let mocked_input_text = build_mocked_input_text();

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, false);
    assert_eq!(2, nodes.len());

    let node = nodes[0].lock().unwrap();
    assert_eq!("あ", node.get_word_info().surface);
    assert_eq!(1, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let node = nodes[1].lock().unwrap();
    assert_eq!("あい", node.get_word_info().surface);
    assert_eq!(2, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, true);
    assert_eq!(0, nodes.len());
  }

  #[test]
  fn test_provide_oov012() {
    let mut plugin = build_plugin();
    let category_info = CategoryInfo {
      is_group: true,
      is_invoke: false,
      length: 2,
    };
    plugin.categories.insert(CategoryType::KANJI, category_info);
    let mocked_input_text = build_mocked_input_text();

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, false);
    assert_eq!(3, nodes.len());

    let node = nodes[0].lock().unwrap();
    assert_eq!("あいう", node.get_word_info().surface);
    assert_eq!(3, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let node = nodes[1].lock().unwrap();
    assert_eq!("あ", node.get_word_info().surface);
    assert_eq!(1, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let node = nodes[2].lock().unwrap();
    assert_eq!("あい", node.get_word_info().surface);
    assert_eq!(2, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, true);
    assert_eq!(0, nodes.len());
  }

  #[test]
  fn test_provide_oov112() {
    let mut plugin = build_plugin();
    let category_info = CategoryInfo {
      is_group: true,
      is_invoke: true,
      length: 2,
    };
    plugin.categories.insert(CategoryType::KANJI, category_info);
    let mocked_input_text = build_mocked_input_text();

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, false);
    assert_eq!(3, nodes.len());

    let node = nodes[0].lock().unwrap();
    assert_eq!("あいう", node.get_word_info().surface);
    assert_eq!(3, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let node = nodes[1].lock().unwrap();
    assert_eq!("あ", node.get_word_info().surface);
    assert_eq!(1, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let node = nodes[2].lock().unwrap();
    assert_eq!("あい", node.get_word_info().surface);
    assert_eq!(2, node.get_word_info().head_word_length);
    assert_eq!(1, node.get_word_info().pos_id);

    let nodes = (&plugin).provide_oov(&mocked_input_text, 0, true);
    assert_eq!(3, nodes.len());
  }
}
