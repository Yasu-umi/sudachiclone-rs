use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IOError};
use std::iter::FromIterator;
use std::num::ParseIntError;
use std::path::Path;
use std::str::FromStr;

use regex::Regex;
use thiserror::Error;

use super::category_type::CategoryType;

#[derive(Clone, Debug)]
struct CharacterCategoryRange {
  pub low: u32,
  pub high: u32,
  pub categories: HashSet<CategoryType>,
}
impl PartialOrd for CharacterCategoryRange {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.high.partial_cmp(&other.high)
  }
}
impl Ord for CharacterCategoryRange {
  fn cmp(&self, other: &Self) -> Ordering {
    self.high.cmp(&other.high)
  }
}
impl Eq for CharacterCategoryRange {}

impl PartialEq for CharacterCategoryRange {
  fn eq(&self, other: &Self) -> bool {
    self.high == other.high
  }
}

impl CharacterCategoryRange {
  pub fn new(low: u32, high: u32, categories: HashSet<CategoryType>) -> Self {
    CharacterCategoryRange {
      low,
      high,
      categories,
    }
  }
}

#[derive(Error, Debug)]
pub enum ReadCharacterDefinitionErr {
  #[error("invalid format at line {0}")]
  InvalidFormatErr(usize),
  #[error("invalid range at line {0}")]
  InvalidRangeErr(usize),
  #[error("{1} is invalid type at line {0}")]
  FoundInvalidTypeErr(usize, String),
  #[error("{self:?}")]
  ParseIntError(#[from] ParseIntError),
  #[error("{self:?}")]
  IOError(#[from] IOError),
}

impl CharacterCategoryRange {
  fn contains(&self, cp: u32) -> bool {
    self.low <= cp && cp < self.high
  }
  pub fn lower(&self, cp: u32) -> bool {
    self.high <= cp
  }
}

fn parse_hex(t: &str) -> Result<u32, ParseIntError> {
  u32::from_str_radix(t.trim_start_matches("0x"), 16)
}

#[derive(Default)]
pub struct CharacterCategory {
  range_list: Vec<CharacterCategoryRange>,
}

impl CharacterCategory {
  pub fn get_category_types(&self, code_point: u32) -> HashSet<CategoryType> {
    let mut start = 0;
    let n = self.range_list.len();
    let mut end = n;
    let mut pivot = (start + end) / 2;
    while pivot < n {
      let range = self.range_list.get(pivot).unwrap();
      if range.contains(code_point) {
        return range.categories.clone();
      }
      if range.lower(code_point) {
        start = pivot;
      } else {
        end = pivot;
      }
      let new_pivot = (start + end) / 2;
      if new_pivot == pivot {
        break;
      }
      pivot = new_pivot;
    }
    let mut set = HashSet::new();
    set.insert(CategoryType::DEFAULT);
    set
  }
  fn compile(&mut self) {
    self.range_list.sort_by_key(|r| r.high);
    self.range_list.sort_by_key(|r| r.low);

    let mut new_range_list = vec![];
    let left_chain: &mut BinaryHeap<Reverse<&CharacterCategoryRange>> = &mut BinaryHeap::new();
    let mut right_chain: Vec<&CharacterCategoryRange> = self.range_list.iter().collect();
    let mut states: Vec<CategoryType> = vec![];
    let mut pivot = 0;
    loop {
      match left_chain.pop() {
        Some(Reverse(left)) => {
          let right = right_chain.get(0);
          let left_end = left.high;
          let right_start = match right {
            Some(r) => r.low,
            None => std::u32::MAX,
          };
          if left_end <= right_start {
            new_range_list.push(CharacterCategoryRange::new(
              pivot,
              left_end,
              HashSet::from_iter(states.iter().cloned()),
            ));
            pivot = left_end;
            for category in left.categories.iter() {
              if let Some(i) = states.iter().position(|c| c == category) {
                states.remove(i);
              }
            }
            continue;
          } else {
            new_range_list.push(CharacterCategoryRange::new(
              pivot,
              right_start,
              HashSet::from_iter(states.iter().cloned()),
            ));
            pivot = right_start;
            if let Some(right) = right {
              states.extend(right.categories.iter());
              left_chain.push(Reverse(&right));
              left_chain.push(Reverse(left));
              right_chain.remove(0);
            }
          }
        }
        None => {
          if right_chain.is_empty() {
            break;
          }
          let right = right_chain.remove(0);
          left_chain.push(Reverse(&right));
          pivot = right.low;
          states.extend(right.categories.iter());
          continue;
        }
      }
    }
    self.range_list = vec![];
    let mut range = new_range_list.remove(0);
    for irange in new_range_list {
      if irange.low == range.high && irange.categories == range.categories {
        range = CharacterCategoryRange::new(range.low, irange.high, range.categories);
      } else {
        self.range_list.push(range);
        range = irange;
      }
    }
    self.range_list.push(range);
  }

  pub fn read_character_definition<P: AsRef<Path>>(
    &mut self,
    char_def: P,
  ) -> Result<&Self, ReadCharacterDefinitionErr> {
    let reader = BufReader::new(File::open(char_def)?);

    let only_spaces = Regex::new(r"^\s*$").unwrap();

    for (index, line) in reader.lines().enumerate() {
      let line = line.unwrap();
      let line_str = line.trim_end();
      if only_spaces.is_match(line_str) || line_str.starts_with('#') {
        continue;
      }
      let cols: Vec<&str> = line_str.split(' ').filter(|s| !s.is_empty()).collect();
      if cols.len() < 2 {
        return Err(ReadCharacterDefinitionErr::InvalidFormatErr(index));
      }
      if !cols[0].contains("0x") {
        continue;
      }
      let r: Vec<&str> = cols[0].split("..").collect();
      let low = parse_hex(r[0])?;
      let mut range = CharacterCategoryRange {
        low,
        high: low + 1,
        categories: HashSet::new(),
      };
      if r.len() > 1 {
        range.high = parse_hex(r[1])? + 1;
      }
      if range.low >= range.high {
        return Err(ReadCharacterDefinitionErr::InvalidRangeErr(index));
      }
      for (j, col) in cols.into_iter().enumerate() {
        if j == 0 {
          continue;
        }
        if col.starts_with('#') || col.is_empty() {
          break;
        }
        match CategoryType::from_str(col) {
          Ok(category_type) => {
            range.categories.insert(category_type);
          }
          Err(_) => {
            return Err(ReadCharacterDefinitionErr::FoundInvalidTypeErr(
              index,
              col.to_string(),
            ));
          }
        }
      }
      self.range_list.push(range);
    }
    self.compile();
    Ok(self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::{remove_file, File};
  use std::io::Write;
  use std::path::PathBuf;
  use std::str::FromStr;

  #[test]
  fn test_get_category_types() {
    let mut category = CharacterCategory::default();
    category
      .read_character_definition(resources_test_dir().join("char.def").as_path())
      .unwrap();
    let code_point = "熙".chars().next().unwrap() as u32;
    assert_eq!(
      CategoryType::KANJI,
      category
        .get_category_types(code_point)
        .into_iter()
        .nth(0)
        .unwrap()
    );
  }

  fn writelines<P: AsRef<Path>>(filename: P, lines: Vec<&str>) -> File {
    let filename = filename.as_ref();
    if filename.is_file() {
      remove_file(filename).unwrap();
    }
    let mut file = File::create(filename).unwrap();
    for line in lines {
      write!(file, "{}", line).unwrap();
    }
    file.flush().unwrap();
    file
  }

  fn resources_test_dir() -> PathBuf {
    PathBuf::from_str(file!())
      .unwrap()
      .parent()
      .unwrap()
      .parent()
      .unwrap()
      .join("resources/test")
  }

  #[test]
  fn test_read_character_definition() {
    let filename = resources_test_dir().join("test_read_character_definition.txt");

    writelines(
      &filename,
      vec![
        "#\n \n",
        "0x0030..0x0039 NUMERIC\n",
        "0x0032         KANJI\n",
      ],
    );
    let mut category = CharacterCategory::default();
    category.read_character_definition(&filename).unwrap();
    assert!(category
      .get_category_types(0x0030)
      .contains(&CategoryType::NUMERIC));
    assert!(category
      .get_category_types(0x0031)
      .contains(&CategoryType::NUMERIC));
    assert!(category
      .get_category_types(0x0032)
      .contains(&CategoryType::NUMERIC));
    assert!(category
      .get_category_types(0x0032)
      .contains(&CategoryType::KANJI));
    assert!(category
      .get_category_types(0x0033)
      .contains(&CategoryType::NUMERIC));
    assert!(category
      .get_category_types(0x0039)
      .contains(&CategoryType::NUMERIC));

    remove_file(&filename).unwrap();

    writelines(
      &filename,
      vec![
        "#\n \n",
        "0x0030..0x0039 NUMERIC\n",
        "0x0070..0x0079 ALPHA\n",
        "0x3007         KANJI\n",
        "0x0030         KANJI\n",
      ],
    );
    let mut category = CharacterCategory::default();
    category.read_character_definition(&filename).unwrap();
    assert!(category
      .get_category_types(0x0030)
      .contains(&CategoryType::NUMERIC));
    assert!(category
      .get_category_types(0x0030)
      .contains(&CategoryType::KANJI));
    assert!(category
      .get_category_types(0x0039)
      .contains(&CategoryType::NUMERIC));
    assert!(category
      .get_category_types(0x3007)
      .contains(&CategoryType::KANJI));
    assert!(category
      .get_category_types(0x0069)
      .contains(&CategoryType::DEFAULT));
    assert!(category
      .get_category_types(0x0070)
      .contains(&CategoryType::ALPHA));
    assert!(category
      .get_category_types(0x0080)
      .contains(&CategoryType::DEFAULT));

    remove_file(&filename).unwrap();

    writelines(
      &filename,
      vec![
        "#\n \n",
        "0x0030..0x0039 KATAKANA\n",
        "0x3007         KANJI KANJINUMERIC\n",
        "0x3008         KANJI KANJINUMERIC\n",
        "0x3009         KANJI KANJINUMERIC\n",
        "0x0039..0x0040 ALPHA\n",
        "0x0030..0x0039 NUMERIC\n",
        "0x0030         KANJI\n",
      ],
    );
    let mut category = CharacterCategory::default();
    category.read_character_definition(&filename).unwrap();
    assert!(category
      .get_category_types(0x0029)
      .contains(&CategoryType::DEFAULT));
    assert!(category
      .get_category_types(0x0030)
      .contains(&CategoryType::NUMERIC));
    assert!(category
      .get_category_types(0x0030)
      .contains(&CategoryType::KATAKANA));
    assert!(category
      .get_category_types(0x0030)
      .contains(&CategoryType::KANJI));

    remove_file(filename).unwrap();
  }

  #[test]
  fn test_read_character_definition_with_invalid_format() {
    let filename =
      resources_test_dir().join("test_read_character_definition_with_invalid_format.txt");

    writelines(&filename, vec!["0x0030..0x0039\n"]);
    let mut category = CharacterCategory::default();
    match category.read_character_definition(&filename) {
      Ok(_) => panic!("should throw invalid format error"),
      Err(err) => assert_eq!("invalid format at line 0", format!("{}", err)),
    }

    remove_file(filename).unwrap();
  }

  #[test]
  fn test_read_character_definition_with_invalid_range() {
    let filename =
      resources_test_dir().join("test_read_character_definition_with_invalid_range.txt");

    writelines(&filename, vec!["0x0030..0x0029 NUMERIC\n"]);
    let mut category = CharacterCategory::default();
    match category.read_character_definition(&filename) {
      Ok(_) => panic!("should throw invalid range error"),
      Err(err) => assert_eq!("invalid range at line 0", format!("{}", err)),
    }

    remove_file(filename).unwrap();
  }

  #[test]
  fn test_read_character_definition_with_invalid_type() {
    let filename =
      resources_test_dir().join("test_read_character_definition_with_invalid_type.txt");

    writelines(&filename, vec!["0x0030..0x0039 FOO\n"]);
    let mut category = CharacterCategory::default();
    match category.read_character_definition(&filename) {
      Ok(_) => panic!("should throw invalid type error"),
      Err(err) => assert_eq!("FOO is invalid type at line 0", format!("{}", err)),
    }

    remove_file(filename).unwrap();
  }
}
