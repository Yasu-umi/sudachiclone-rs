use std::hash::Hash;
use std::str::FromStr;

use thiserror::Error;

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum CategoryType {
  Default = 1,
  Space = 1 << 1,
  Kanji = 1 << 2,
  Symbol = 1 << 3,
  Numeric = 1 << 4,
  Alpha = 1 << 5,
  Hiragana = 1 << 6,
  Katakana = 1 << 7,
  KanjiNumeric = 1 << 8,
  Greek = 1 << 9,
  Cyrillic = 1 << 10,
  User1 = 1 << 11,
  User2 = 1 << 12,
  User3 = 1 << 13,
  User4 = 1 << 14,
  Nooovbow = 1 << 15,
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
      "DEFAULT" => Ok(CategoryType::Default),
      "SPACE" => Ok(CategoryType::Space),
      "KANJI" => Ok(CategoryType::Kanji),
      "SYMBOL" => Ok(CategoryType::Symbol),
      "NUMERIC" => Ok(CategoryType::Numeric),
      "ALPHA" => Ok(CategoryType::Alpha),
      "HIRAGANA" => Ok(CategoryType::Hiragana),
      "KATAKANA" => Ok(CategoryType::Katakana),
      "KANJINUMERIC" => Ok(CategoryType::KanjiNumeric),
      "GREEK" => Ok(CategoryType::Greek),
      "CYRILLIC" => Ok(CategoryType::Cyrillic),
      "USER1" => Ok(CategoryType::User1),
      "USER2" => Ok(CategoryType::User2),
      "USER3" => Ok(CategoryType::User3),
      "USER4" => Ok(CategoryType::User4),
      "NOOOVBOW" => Ok(CategoryType::Nooovbow),
      _ => Err(CategoryTypeErr::CategoryTypeKeyErr(key.to_string())),
    }
  }
}
