use std::hash::Hash;
use std::str::FromStr;

use thiserror::Error;

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum CategoryType {
  DEFAULT = 1,
  SPACE = 1 << 1,
  KANJI = 1 << 2,
  SYMBOL = 1 << 3,
  NUMERIC = 1 << 4,
  ALPHA = 1 << 5,
  HIRAGANA = 1 << 6,
  KATAKANA = 1 << 7,
  KANJINUMERIC = 1 << 8,
  GREEK = 1 << 9,
  CYRILLIC = 1 << 10,
  USER1 = 1 << 11,
  USER2 = 1 << 12,
  USER3 = 1 << 13,
  USER4 = 1 << 14,
  NOOOVBOW = 1 << 15,
}

#[derive(Error, Debug)]
pub enum CategoryTypeErr {
  #[error("key error {0}")]
  CategoryTypeKeyErr(String),
}

impl FromStr for CategoryType {
  type Err = CategoryTypeErr;
  fn from_str(key: &str) -> Result<Self, Self::Err> {
    match key {
      "DEFAULT" => Ok(CategoryType::DEFAULT),
      "SPACE" => Ok(CategoryType::SPACE),
      "KANJI" => Ok(CategoryType::KANJI),
      "SYMBOL" => Ok(CategoryType::SYMBOL),
      "NUMERIC" => Ok(CategoryType::NUMERIC),
      "ALPHA" => Ok(CategoryType::ALPHA),
      "HIRAGANA" => Ok(CategoryType::HIRAGANA),
      "KATAKANA" => Ok(CategoryType::KATAKANA),
      "KANJINUMERIC" => Ok(CategoryType::KANJINUMERIC),
      "GREEK" => Ok(CategoryType::GREEK),
      "CYRILLIC" => Ok(CategoryType::CYRILLIC),
      "USER1" => Ok(CategoryType::USER1),
      "USER2" => Ok(CategoryType::USER2),
      "USER3" => Ok(CategoryType::USER3),
      "USER4" => Ok(CategoryType::USER4),
      "NOOOVBOW" => Ok(CategoryType::NOOOVBOW),
      _ => Err(CategoryTypeErr::CategoryTypeKeyErr(key.to_string())),
    }
  }
}
