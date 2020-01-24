use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Range;
use std::rc::Rc;

use thiserror::Error;

use super::dictionary_lib::category_type::CategoryType;
use super::dictionary_lib::grammar::{GetCharacterCategory, Grammar};
use super::utf8_input_text::UTF8InputText;

pub struct UTF8InputTextBuilder<G = Rc<RefCell<Grammar>>> {
  grammar: G,
  original_text: String,
  modified_text: String,
  text_offsets: Vec<usize>,
}

#[derive(Error, Debug)]
pub enum ReplaceErr {
  #[error("{0}")]
  RangeErr(String),
}

impl<G> UTF8InputTextBuilder<G> {
  pub fn new(text: &str, grammar: G) -> UTF8InputTextBuilder<G> {
    UTF8InputTextBuilder {
      grammar,
      original_text: text.to_string(),
      modified_text: text.to_string(),
      text_offsets: (0..=text.to_string().chars().count()).collect(),
    }
  }
  pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<(), ReplaceErr> {
    let mut range = range;
    let modified_text_chars: Vec<char> = self.modified_text.chars().collect();
    if range.start > modified_text_chars.len() {
      return Err(ReplaceErr::RangeErr(String::from("start > length")));
    }
    if range.start > range.end {
      return Err(ReplaceErr::RangeErr(String::from("start > end")));
    }
    if range.start == range.end {
      return Err(ReplaceErr::RangeErr(String::from("start == end")));
    }
    if range.end > modified_text_chars.len() {
      range.end = modified_text_chars.len();
    }
    self.modified_text = modified_text_chars[..range.start]
      .iter()
      .chain(text.chars().collect::<Vec<char>>().iter())
      .chain(modified_text_chars[range.end..].iter())
      .collect();

    let offset = self.text_offsets[range.start];
    let len = text.chars().count();
    if range.end - range.start > len {
      self.text_offsets = self
        .text_offsets
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < (range.start + len) || range.end <= *i)
        .map(|(_, offset)| *offset)
        .collect();
    }
    for i in 0..len {
      if range.start + i < range.end {
        self.text_offsets[range.start + i] = offset;
      } else {
        self.text_offsets.insert(range.start + i, offset);
      }
    }
    Ok(())
  }
  pub fn get_original_text(&self) -> String {
    self.original_text.clone()
  }
  pub fn get_text(&self) -> String {
    self.modified_text.clone()
  }
}

impl<G: GetCharacterCategory> UTF8InputTextBuilder<Rc<RefCell<G>>> {
  pub fn build(self) -> UTF8InputText {
    let modified_text = self.get_text();
    let bytes = modified_text.clone().into_bytes();
    let len = bytes.len();
    let mut byte_indexes = vec![0; len + 1];
    let mut offsets = vec![0; len + 1];

    let mut j = 0;
    for i in 0..self.modified_text.chars().count() {
      // 注: サロゲートペア文字は考慮していない
      for _ in 0..self.modified_text.chars().nth(i).unwrap().len_utf8() {
        byte_indexes[j] = i;
        offsets[j] = self.text_offsets[i];
        j += 1;
      }
    }
    byte_indexes[len] = modified_text.chars().count();
    offsets[len] = *self.text_offsets.last().unwrap();

    let char_categories = self.get_char_category_types(&modified_text);
    let char_category_continuities =
      get_char_category_continuities(&modified_text, &char_categories);
    let can_bow_list = build_can_bow_list(&modified_text, &char_categories);

    UTF8InputText::new(
      self.original_text,
      modified_text,
      bytes,
      offsets,
      byte_indexes,
      char_categories,
      char_category_continuities,
      can_bow_list,
    )
  }
  fn get_char_category_types(&self, text: &str) -> Vec<HashSet<CategoryType>> {
    text
      .chars()
      .map(|c| {
        self
          .grammar
          .borrow()
          .get_character_category()
          .as_ref()
          .unwrap()
          .get_category_types(c as u32)
      })
      .collect()
  }
}

fn build_can_bow_list(text: &str, char_categories: &[HashSet<CategoryType>]) -> Vec<bool> {
  if text.is_empty() {
    return vec![];
  }
  let mut can_bow_list = vec![];
  for (i, cat) in char_categories.iter().enumerate() {
    if i == 0 {
      can_bow_list.push(true);
      continue;
    }
    if cat.contains(&CategoryType::ALPHA)
      || cat.contains(&CategoryType::GREEK)
      || cat.contains(&CategoryType::CYRILLIC)
    {
      can_bow_list.push(cat.intersection(&char_categories[i - 1]).next().is_none());
      continue;
    }
    can_bow_list.push(true);
  }
  can_bow_list
}

fn get_char_category_continuities(
  text: &str,
  char_categories: &[HashSet<CategoryType>],
) -> Vec<usize> {
  if text.chars().count() == 0 {
    return vec![];
  }
  let mut char_category_continuities = vec![];
  let mut i = 0;
  while i < char_categories.len() {
    let next = i + get_char_category_continuous_length(char_categories, i);
    let mut len = 0;
    for j in i..next {
      len += text.chars().nth(j).unwrap().len_utf8();
    }
    for k in 0..len {
      let k = len - k;
      char_category_continuities.push(k);
    }
    i = next;
  }
  char_category_continuities
}

fn get_char_category_continuous_length(
  char_categories: &[HashSet<CategoryType>],
  offset: usize,
) -> usize {
  let mut continuous_category = char_categories[offset].clone();
  for len in 1..char_categories.len() - offset {
    continuous_category = continuous_category
      .intersection(&char_categories[offset + len])
      .cloned()
      .collect();
    if continuous_category.is_empty() {
      return len;
    }
  }
  char_categories.len() - offset
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dictionary_lib::category_type::CategoryType;
  use crate::dictionary_lib::character_category::CharacterCategory;
  use crate::dictionary_lib::grammar::SetCharacterCategory;
  use crate::utf8_input_text::InputText;
  use std::path::PathBuf;
  use std::str::FromStr;

  const TEXT: &str = "âｂC1あ234漢字𡈽アｺﾞ";

  struct MockGrammar {
    character_category: Option<CharacterCategory>,
  }
  impl MockGrammar {
    fn new() -> MockGrammar {
      MockGrammar {
        character_category: None,
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

  fn build_builder() -> UTF8InputTextBuilder<Rc<RefCell<MockGrammar>>> {
    let mut character_category = CharacterCategory::default();
    character_category
      .read_character_definition(
        PathBuf::from_str(file!())
          .unwrap()
          .parent()
          .unwrap()
          .join("resources/char.def"),
      )
      .unwrap();
    let mut grammar = MockGrammar::new();
    grammar.set_character_category(Some(character_category));
    UTF8InputTextBuilder::new(TEXT, Rc::new(RefCell::new(grammar)))
  }

  #[test]
  fn test_get_original_text() {
    let builder = build_builder();
    assert_eq!(builder.get_original_text(), TEXT);
    assert_eq!(builder.get_text(), TEXT);
    let input = builder.build();
    assert_eq!(input.get_original_text(), TEXT);
    assert_eq!(input.get_text(), TEXT);
  }

  #[test]
  fn test_get_byte_text() {
    let builder = build_builder();
    let input = builder.build();
    assert_eq!(input.get_byte_text().len(), 32);
    assert_eq!(TEXT.as_bytes(), input.get_byte_text().as_slice());
  }

  #[test]
  fn test_get_original_index() {
    let builder = build_builder();
    let input = builder.build();
    assert_eq!(input.get_original_index(0), 0);
    assert_eq!(input.get_original_index(1), 0);
    assert_eq!(input.get_original_index(2), 1);
    assert_eq!(input.get_original_index(4), 1);
    assert_eq!(input.get_original_index(6), 3);
    assert_eq!(input.get_original_index(7), 4);
    assert_eq!(input.get_original_index(10), 5);
    assert_eq!(input.get_original_index(18), 9);
    assert_eq!(input.get_original_index(19), 10);
    assert_eq!(input.get_original_index(22), 10);
    assert_eq!(input.get_original_index(23), 11);
    assert_eq!(input.get_original_index(28), 12);
    assert_eq!(input.get_original_index(31), 13);
  }

  #[test]
  fn test_get_char_category_types() {
    let builder = build_builder();
    let input = builder.build();
    assert!(input
      .get_char_category_types(0, None)
      .contains(&CategoryType::ALPHA));
    assert!(input
      .get_char_category_types(2, None)
      .contains(&CategoryType::ALPHA));
    assert!(input
      .get_char_category_types(5, None)
      .contains(&CategoryType::ALPHA));
    assert!(input
      .get_char_category_types(6, None)
      .contains(&CategoryType::NUMERIC));
    assert!(input
      .get_char_category_types(7, None)
      .contains(&CategoryType::HIRAGANA));
    assert!(input
      .get_char_category_types(9, None)
      .contains(&CategoryType::HIRAGANA));
    assert!(input
      .get_char_category_types(10, None)
      .contains(&CategoryType::NUMERIC));
    assert!(input
      .get_char_category_types(13, None)
      .contains(&CategoryType::KANJI));
    assert!(input
      .get_char_category_types(18, None)
      .contains(&CategoryType::KANJI));
    assert!(input
      .get_char_category_types(19, None)
      .contains(&CategoryType::DEFAULT));
    assert!(input
      .get_char_category_types(22, None)
      .contains(&CategoryType::DEFAULT));
    assert!(input
      .get_char_category_types(23, None)
      .contains(&CategoryType::KATAKANA));
    assert!(input
      .get_char_category_types(26, None)
      .contains(&CategoryType::KATAKANA));
    assert!(input
      .get_char_category_types(31, None)
      .contains(&CategoryType::KATAKANA));
  }

  #[test]
  fn test_get_char_category_continuous_length() {
    let builder = build_builder();
    let input = builder.build();
    assert_eq!(input.get_char_category_continuous_length(0), 6);
    assert_eq!(input.get_char_category_continuous_length(1), 5);
    assert_eq!(input.get_char_category_continuous_length(2), 4);
    assert_eq!(input.get_char_category_continuous_length(5), 1);
    assert_eq!(input.get_char_category_continuous_length(6), 1);
    assert_eq!(input.get_char_category_continuous_length(7), 3);
    assert_eq!(input.get_char_category_continuous_length(10), 3);
    assert_eq!(input.get_char_category_continuous_length(11), 2);
    assert_eq!(input.get_char_category_continuous_length(12), 1);
    assert_eq!(input.get_char_category_continuous_length(19), 4);
    assert_eq!(input.get_char_category_continuous_length(22), 1);
    assert_eq!(input.get_char_category_continuous_length(23), 9);
    assert_eq!(input.get_char_category_continuous_length(26), 6);
    assert_eq!(input.get_char_category_continuous_length(31), 1);
  }

  #[test]
  fn test_replace_with_same_length() {
    let mut builder = build_builder();
    builder.replace(8..10, "ああ").unwrap();
    assert_eq!(builder.get_original_text(), TEXT);
    assert_eq!(builder.get_text(), "âｂC1あ234ああ𡈽アｺﾞ");
    let input = builder.build();
    assert_eq!(input.get_original_text(), TEXT);
    assert_eq!(input.get_text(), "âｂC1あ234ああ𡈽アｺﾞ");
    assert_eq!(input.get_byte_text().len(), 32);
    assert_eq!(input.get_original_index(0), 0);
    assert_eq!(input.get_original_index(12), 7);
    assert_eq!(input.get_original_index(13), 8);
    assert_eq!(input.get_original_index(15), 8);
    assert_eq!(input.get_original_index(16), 8);
    assert_eq!(input.get_original_index(18), 8);
    assert_eq!(input.get_original_index(19), 10);
    assert_eq!(input.get_original_index(22), 10);
    assert_eq!(input.get_original_index(31), 13);
  }

  #[test]
  fn test_replace_with_deletion() {
    let mut builder = build_builder();
    builder.replace(8..10, "あ").unwrap();
    assert_eq!(builder.get_original_text(), TEXT);
    assert_eq!(builder.get_text(), "âｂC1あ234あ𡈽アｺﾞ");
    let input = builder.build();
    assert_eq!(input.get_original_text(), TEXT);
    assert_eq!(input.get_text(), "âｂC1あ234あ𡈽アｺﾞ");
    assert_eq!(input.get_byte_text().len(), 29);
    assert_eq!(input.get_original_index(0), 0);
    assert_eq!(input.get_original_index(12), 7);
    assert_eq!(input.get_original_index(13), 8);
    assert_eq!(input.get_original_index(15), 8);
    assert_eq!(input.get_original_index(16), 10);
    assert_eq!(input.get_original_index(19), 10);
    assert_eq!(input.get_original_index(28), 13);
  }

  #[test]
  fn test_replace_with_insertion() {
    let mut builder = build_builder();
    builder.replace(8..10, "あああ").unwrap();
    assert_eq!(builder.get_original_text(), TEXT);
    assert_eq!(builder.get_text(), "âｂC1あ234あああ𡈽アｺﾞ");
    let input = builder.build();
    assert_eq!(input.get_original_text(), TEXT);
    assert_eq!(input.get_text(), "âｂC1あ234あああ𡈽アｺﾞ");
    assert_eq!(input.get_byte_text().len(), 35);
    assert_eq!(input.get_original_index(0), 0);
    assert_eq!(input.get_original_index(12), 7);
    assert_eq!(input.get_original_index(13), 8);
    assert_eq!(input.get_original_index(21), 8);
    assert_eq!(input.get_original_index(22), 10);
    assert_eq!(input.get_original_index(25), 10);
    assert_eq!(input.get_original_index(35), 14);
  }

  #[test]
  fn test_replace_multi_times() {
    let mut builder = build_builder();
    builder.replace(0..1, "a").unwrap();
    builder.replace(1..2, "b").unwrap();
    builder.replace(2..3, "c").unwrap();
    builder.replace(10..11, "土").unwrap();
    builder.replace(12..14, "ゴ").unwrap();
    let input = builder.build();
    assert_eq!(input.get_original_text(), TEXT);
    assert_eq!(input.get_text(), "abc1あ234漢字土アゴ");
    assert_eq!(input.get_byte_text().len(), 25);
    assert_eq!(input.get_original_index(0), 0);
    assert_eq!(input.get_original_index(1), 1);
    assert_eq!(input.get_original_index(2), 2);
    assert_eq!(input.get_original_index(7), 5);
    assert_eq!(input.get_original_index(8), 6);
    assert_eq!(input.get_original_index(9), 7);
    assert_eq!(input.get_original_index(15), 9);
    assert_eq!(input.get_original_index(16), 10);
    assert_eq!(input.get_original_index(18), 10);
    assert_eq!(input.get_original_index(19), 11);
    assert_eq!(input.get_original_index(21), 11);
    assert_eq!(input.get_original_index(22), 12);
    assert_eq!(input.get_original_index(24), 12);
  }

  #[test]
  fn test_get_byte_length_by_code_points() {
    let builder = build_builder();
    let input = builder.build();
    assert_eq!(input.get_code_points_offset_length(0, 1), 2);
    assert_eq!(input.get_code_points_offset_length(0, 4), 7);
    assert_eq!(input.get_code_points_offset_length(10, 1), 1);
    assert_eq!(input.get_code_points_offset_length(11, 1), 1);
    assert_eq!(input.get_code_points_offset_length(12, 1), 1);
    assert_eq!(input.get_code_points_offset_length(13, 2), 6);
    assert_eq!(input.get_code_points_offset_length(19, 1), 4);
    assert_eq!(input.get_code_points_offset_length(23, 3), 9);
  }

  #[test]
  fn test_get_byte_length_by_code_point_count() {
    let builder = build_builder();
    let input = builder.build();
    assert_eq!(input.code_point_count(0..2), 1);
    assert_eq!(input.code_point_count(0..7), 4);
    assert_eq!(input.code_point_count(13..19), 2);
  }

  #[test]
  fn test_can_bow() {
    let builder = build_builder();
    let input = builder.build();
    assert!(input.can_bow(0)); // â
    assert!(!input.can_bow(1));
    assert!(!input.can_bow(2)); // ｂ
    assert!(!input.can_bow(3));
    assert!(!input.can_bow(4));
    assert!(!input.can_bow(5)); // C
    assert!(input.can_bow(6)); // 1
    assert!(input.can_bow(7)); // あ

    assert!(input.can_bow(19)); // 𡈽
    assert!(!input.can_bow(20));
    assert!(!input.can_bow(21));
    assert!(!input.can_bow(22));
    assert!(input.can_bow(23)); // ア
  }

  #[test]
  fn test_get_word_candidate_length() {
    let builder = build_builder();
    let input = builder.build();
    assert_eq!(input.get_word_candidate_length(0), 6);
    assert_eq!(input.get_word_candidate_length(6), 1);
    assert_eq!(input.get_word_candidate_length(19), 4);
    assert_eq!(input.get_word_candidate_length(29), 3);
  }
}
