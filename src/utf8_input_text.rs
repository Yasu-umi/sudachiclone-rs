use std::borrow::Cow;
use std::collections::HashSet;
use std::ops::Range;

use thiserror::Error;

use super::dictionary_lib::category_type::CategoryType;

#[derive(Error, Debug)]
pub enum InputTextErr {
  #[error("{0}")]
  FailedSubstringErr(String),
}

pub struct Utf8InputText {
  original_text: String,
  modified_text: String,
  bytes: Vec<u8>,
  offsets: Vec<usize>,
  byte_indexes: Vec<usize>,
  char_categories: Vec<HashSet<CategoryType>>,
  char_category_continuities: Vec<usize>,
  can_bow_list: Vec<bool>,
}

pub trait InputText {
  fn get_char_category_continuous_length(&self, index: usize) -> usize;
  fn get_char_category_types(&self, start: usize, end: Option<usize>) -> HashSet<CategoryType>;
  fn get_substring(&self, start: usize, end: usize) -> Result<Cow<str>, InputTextErr>;
  fn get_code_points_offset_length(&self, index: usize, code_point_offset: usize) -> usize;
  fn get_word_candidate_length(&self, index: usize) -> usize;
}

impl Utf8InputText {
  pub fn new(
    original_text: String,
    modified_text: String,
    bytes: Vec<u8>,
    offsets: Vec<usize>,
    byte_indexes: Vec<usize>,
    char_categories: Vec<HashSet<CategoryType>>,
    char_category_continuities: Vec<usize>,
    can_bow_list: Vec<bool>,
  ) -> Utf8InputText {
    Utf8InputText {
      original_text,
      modified_text,
      bytes,
      offsets,
      byte_indexes,
      char_categories,
      char_category_continuities,
      can_bow_list,
    }
  }
  pub fn get_original_text(&self) -> &String {
    &self.original_text
  }
  pub fn get_text(&self) -> &String {
    &self.modified_text
  }
  pub fn get_byte_text(&self) -> &Vec<u8> {
    &self.bytes
  }
  fn get_offset_text_length(&self, index: usize) -> usize {
    self.byte_indexes[index]
  }
  fn is_char_alignment(&self, index: usize) -> bool {
    (self.bytes[index] & 0xC0) != 0x80
  }
  pub fn get_original_index(&self, index: usize) -> usize {
    self.offsets[index]
  }
  pub fn can_bow(&self, idx: usize) -> bool {
    self.is_char_alignment(idx) && self.can_bow_list[self.get_offset_text_length(idx)]
  }
  pub fn code_point_count(&self, range: Range<usize>) -> usize {
    self.get_offset_text_length(range.end) - self.get_offset_text_length(range.start)
  }
}

impl InputText for Utf8InputText {
  fn get_substring(&self, start: usize, end: usize) -> Result<Cow<str>, InputTextErr> {
    if end > self.bytes.len() {
      return Err(InputTextErr::FailedSubstringErr(String::from("end > self.bytes.len()")));
    }
    if start > end {
      return Err(InputTextErr::FailedSubstringErr(String::from("start > end")));
    }
    Ok(Cow::Borrowed(self.modified_text.get(start..end).unwrap()))
  }
  fn get_char_category_continuous_length(&self, index: usize) -> usize {
    self.char_category_continuities[index]
  }
  fn get_code_points_offset_length(&self, index: usize, code_point_offset: usize) -> usize {
    let mut length = 0;
    let target = self.get_offset_text_length(index) + code_point_offset;
    for i in index..self.bytes.len() {
      if self.get_offset_text_length(i) >= target {
        return length;
      }
      length += 1;
    }
    length
  }
  fn get_char_category_types(&self, start: usize, end: Option<usize>) -> HashSet<CategoryType> {
    match end {
      Some(end) => {
        if start + self.get_char_category_continuous_length(start) < end {
          let mut set = HashSet::new();
          set.insert(CategoryType::Default);
          return set;
        }
        let start = self.get_offset_text_length(start);
        let end = self.get_offset_text_length(end);
        let mut continuous_category = self.char_categories[start].clone();
        for i in start + 1..end {
          continuous_category = continuous_category
            .intersection(&self.char_categories[i])
            .cloned()
            .collect();
        }
        continuous_category
      }
      None => self.char_categories[self.get_offset_text_length(start)].clone(),
    }
  }
  fn get_word_candidate_length(&self, index: usize) -> usize {
    for i in index + 1..self.bytes.len() {
      if self.can_bow(i) {
        return i - index;
      }
    }
    self.bytes.len() - index
  }
}
