use std::collections::HashSet;

use serde_json::Value;

use super::input_text_plugin::{
  InputTextPlugin, InputTextPluginReplaceErr, InputTextPluginSetupErr,
};
use crate::utf8_input_text_builder::UTF8InputTextBuilder;

#[derive(Debug)]
pub struct ProlongedSoundMarkInputTextPlugin {
  psm_set: HashSet<u32>,
  replace_symbol: String,
}

impl<G> InputTextPlugin<G> for ProlongedSoundMarkInputTextPlugin {
  fn setup(&mut self) -> Result<(), InputTextPluginSetupErr> {
    Ok(())
  }
  fn rewrite(
    &self,
    builder: &mut UTF8InputTextBuilder<G>,
  ) -> Result<(), InputTextPluginReplaceErr> {
    let text = builder.get_text();
    let n = text.chars().count();
    let mut offset = 0;
    let mut is_psm = false;
    let mut m_start_idx = n;
    for (i, c) in text.chars().enumerate() {
      let cp = c as u32;
      if !is_psm && self.psm_set.contains(&cp) {
        is_psm = true;
        m_start_idx = i;
      } else if is_psm && !self.psm_set.contains(&cp) {
        if i - m_start_idx > 1 {
          builder.replace(m_start_idx - offset..i - offset, &self.replace_symbol)?;
          offset += i - m_start_idx - 1;
        }
        is_psm = false;
      }
    }
    if is_psm && n - m_start_idx > 1 {
      builder.replace(m_start_idx - offset..n - offset, &self.replace_symbol)?;
    }
    Ok(())
  }
}

impl ProlongedSoundMarkInputTextPlugin {
  pub fn new(json_obj: &Value) -> ProlongedSoundMarkInputTextPlugin {
    let mut psm_set = HashSet::new();
    if let Some(Value::Array(marks)) = json_obj.get("prolongedSoundMarks") {
      for mark in marks {
        if let Value::String(psm) = mark {
          psm_set.insert(psm.chars().nth(0).unwrap() as u32);
        }
      }
    }
    let mut replace_symbol = String::from("ー");
    if let Some(Value::String(_replacement_symbol)) = json_obj.get("replacementSymbol") {
      replace_symbol = _replacement_symbol.to_string();
    }
    ProlongedSoundMarkInputTextPlugin {
      psm_set,
      replace_symbol,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dictionary_lib::character_category::CharacterCategory;
  use crate::dictionary_lib::grammar::{GetCharacterCategory, SetCharacterCategory};
  use serde_json::json;
  use std::cell::RefCell;
  use std::path::PathBuf;
  use std::rc::Rc;
  use std::str::FromStr;

  struct MockGrammar {
    character_category: Option<CharacterCategory>,
  }
  impl MockGrammar {
    fn new() -> MockGrammar {
      let mut character_category = CharacterCategory::default();
      character_category
        .read_character_definition(
          PathBuf::from_str(file!())
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("resources/char.def")
            .as_path(),
        )
        .unwrap();
      MockGrammar {
        character_category: Some(character_category),
      }
    }
  }
  impl GetCharacterCategory for MockGrammar {
    fn get_character_category(&self) -> &Option<CharacterCategory> {
      &self.character_category
    }
  }
  impl SetCharacterCategory for MockGrammar {
    fn set_character_category(&mut self, character_category: Option<CharacterCategory>) {
      self.character_category = character_category;
    }
  }

  fn build_plugin() -> ProlongedSoundMarkInputTextPlugin {
    ProlongedSoundMarkInputTextPlugin::new(&json!({"prolongedSoundMarks":["ー", "〜", "〰"]}))
  }

  #[test]
  fn test_combine_continuous_prolonged_sound_mark() {
    let original = "ゴーール";
    let normalized = "ゴール";
    let plugin = build_plugin();
    let mut builder =
      UTF8InputTextBuilder::new(original, Rc::new(RefCell::new(MockGrammar::new())));
    plugin.rewrite(&mut builder).unwrap();
    let text = builder.build();

    assert_eq!(original, text.get_original_text());
    assert_eq!(normalized, text.get_text());
    let bytes = text.get_byte_text();
    assert_eq!(9, bytes.len());

    assert_eq!(
      &vec![0xe3, 0x82, 0xb4, 0xe3, 0x83, 0xbc, 0xe3, 0x83, 0xab],
      bytes
    );
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(1, text.get_original_index(3));
    assert_eq!(3, text.get_original_index(6));
    assert_eq!(4, text.get_original_index(9));
  }

  #[test]
  fn test_combined_continuous_prolonged_sound_marks_at_end() {
    let original = "スーパーー";
    let normalized = "スーパー";
    let plugin = build_plugin();
    let mut builder =
      UTF8InputTextBuilder::new(original, Rc::new(RefCell::new(MockGrammar::new())));
    plugin.rewrite(&mut builder).unwrap();
    let text = builder.build();

    assert_eq!(original, text.get_original_text());
    assert_eq!(normalized, text.get_text());
    let bytes = text.get_byte_text();
    assert_eq!(12, bytes.len());

    assert_eq!(
      &vec![0xe3, 0x82, 0xb9, 0xe3, 0x83, 0xbc, 0xe3, 0x83, 0x91, 0xe3, 0x83, 0xbc],
      bytes
    );
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(1, text.get_original_index(3));
    assert_eq!(2, text.get_original_index(6));
    assert_eq!(3, text.get_original_index(9));
    assert_eq!(5, text.get_original_index(12));
  }

  #[test]
  fn test_combine_continuous_prolonged_sound_marks_multi_times() {
    let original = "エーービーーーシーーーー";
    let normalized = "エービーシー";
    let plugin = build_plugin();
    let mut builder =
      UTF8InputTextBuilder::new(original, Rc::new(RefCell::new(MockGrammar::new())));
    plugin.rewrite(&mut builder).unwrap();
    let text = builder.build();

    assert_eq!(original, text.get_original_text());
    assert_eq!(normalized, text.get_text());
    let bytes = text.get_byte_text();
    assert_eq!(18, bytes.len());

    assert_eq!(
      &vec![
        0xe3, 0x82, 0xa8, 0xe3, 0x83, 0xbc, 0xe3, 0x83, 0x93, 0xe3, 0x83, 0xbc, 0xe3, 0x82, 0xb7,
        0xe3, 0x83, 0xbc
      ],
      bytes
    );
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(1, text.get_original_index(3));
    assert_eq!(3, text.get_original_index(6));
    assert_eq!(4, text.get_original_index(9));
    assert_eq!(7, text.get_original_index(12));
    assert_eq!(8, text.get_original_index(15));
    assert_eq!(12, text.get_original_index(18));
  }

  #[test]
  fn test_combine_continuous_prolonged_sound_marks_multi_symbol_types() {
    let original = "エーービ〜〜〜シ〰〰〰〰";
    let normalized = "エービーシー";
    let plugin = build_plugin();
    let mut builder =
      UTF8InputTextBuilder::new(original, Rc::new(RefCell::new(MockGrammar::new())));
    plugin.rewrite(&mut builder).unwrap();
    let text = builder.build();

    assert_eq!(original, text.get_original_text());
    assert_eq!(normalized, text.get_text());
    let bytes = text.get_byte_text();
    assert_eq!(18, bytes.len());

    assert_eq!(
      &vec![
        0xe3, 0x82, 0xa8, 0xe3, 0x83, 0xbc, 0xe3, 0x83, 0x93, 0xe3, 0x83, 0xbc, 0xe3, 0x82, 0xb7,
        0xe3, 0x83, 0xbc
      ],
      bytes
    );
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(1, text.get_original_index(3));
    assert_eq!(3, text.get_original_index(6));
    assert_eq!(4, text.get_original_index(9));
    assert_eq!(7, text.get_original_index(12));
    assert_eq!(8, text.get_original_index(15));
    assert_eq!(12, text.get_original_index(18));
  }
}
