use std::io::Error as IOError;

use thiserror::Error;

use super::word_info::WordInfo;

pub trait Lexicon {
  fn lookup(&self, text: &[u8], offset: usize) -> Vec<(usize, usize)>;
  fn get_left_id(&self, word_id: usize) -> i16;
  fn get_right_id(&self, word_id: usize) -> i16;
  fn get_cost(&self, word_id: usize) -> i16;
  fn get_word_info(&self, word_id: usize) -> WordInfo;
  fn get_dictionary_id(&self, word_id: usize) -> usize;
}

pub trait Size {
  fn size(&self) -> usize;
}

pub trait GetWordId {
  fn get_word_id(
    &self,
    headword: &str,
    pos_id: u16,
    reading_form: &str,
  ) -> Result<usize, LexiconErr>;
}

#[derive(Error, Debug)]
pub enum LexiconErr {
  #[error("not found word id")]
  NotFoundWordIdErr,
  #[error("{self:?}")]
  IOError(#[from] IOError),
}
