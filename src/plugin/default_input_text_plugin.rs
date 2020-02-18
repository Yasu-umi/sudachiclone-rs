use std::cell::RefCell;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IOError};
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;

use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use super::input_text_plugin::{InputTextPlugin, InputTextPluginReplaceErr};
use crate::config::Config;
use crate::dictionary_lib::grammar::Grammar;
use crate::utf8_input_text_builder::UTF8InputTextBuilder;

type KeyLengths = HashMap<char, usize>;
type ReplaceCharMap = HashMap<Vec<u8>, String>;
type IgnoreNormalizeSet = HashSet<String>;

pub struct DefaultInputTextPlugin<G = Rc<RefCell<Grammar>>> {
  phantom: PhantomData<G>,
  key_lengths: KeyLengths,
  replace_char_map: ReplaceCharMap,
  ignore_normalize_set: IgnoreNormalizeSet,
}

impl<G> InputTextPlugin<G> for DefaultInputTextPlugin<G> {
  fn rewrite(
    &self,
    builder: &mut UTF8InputTextBuilder<G>,
  ) -> Result<(), InputTextPluginReplaceErr> {
    let mut offset: i32 = 0;
    let mut next_offset: i32 = 0;
    let text = builder.get_text();

    let mut i: i32 = -1;
    loop {
      i += 1;
      let i_us = i as usize;
      let count = text.chars().count();
      if i_us >= text.chars().count() {
        break;
      }
      let mut textloop = false;
      offset += next_offset;
      next_offset = 0;
      let original = text.chars().nth(i_us).unwrap();

      // 1. replace char without normalize
      let max_length = min(*self.key_lengths.get(&original).unwrap_or(&0), count - 1);
      for l in 0..max_length {
        let l = max_length - l;
        let chars: Vec<char> = text.chars().collect();
        let buf: &Vec<u8> = &chars[i_us..(i_us + l)]
          .iter()
          .map(|c| {
            let mut buf = vec![0; c.len_utf8()];
            c.encode_utf8(&mut buf);
            buf
          })
          .flatten()
          .collect();
        if let Some(replace) = self.replace_char_map.get(buf) {
          builder.replace(
            ((i + offset) as usize)..(i + l as i32 + offset) as usize,
            replace,
          )?;
          next_offset += (replace.chars().count() as i32) - (l as i32);
          i += l as i32 - 1;
          textloop = true;
          break;
        }
      }
      if textloop {
        continue;
      }
      // 2. normalize
      // 2-1. capital alphabet (not only Latin but Greek, Cyrillic, etc.) -> small
      let original = original.to_string();
      let lower = original.to_lowercase();
      let replace = if self.ignore_normalize_set.contains(&lower) {
        if original == lower {
          continue;
        }
        lower
      } else {
        // 2-2. normalize (except in ignoreNormalize)
        // e.g. full-width alphabet -> half-width / ligature / etc.
        lower.nfkc().collect::<String>()
      };
      next_offset = (replace.chars().count() as i32) - 1;
      if original != replace {
        builder.replace((i + offset) as usize..(i + 1 + offset) as usize, &replace)?;
      }
    }
    Ok(())
  }
}

#[derive(Error, Debug)]
pub enum DefaultInputTextPluginSetupErr {
  #[error("{1} is not character at line {0}")]
  NotCharacterErr(usize, String),
  #[error("{1} is already defined at line {0}")]
  AlreadyDefinedErr(usize, String),
  #[error("invalid format at line {0}")]
  InvalidFormatErr(usize),
  #[error("{self:?}")]
  IOError(#[from] IOError),
  #[error("{self:?}")]
  Infallible(#[from] Infallible),
}

impl<G> DefaultInputTextPlugin<G> {
  pub fn setup(
    config: &Config,
  ) -> Result<DefaultInputTextPlugin<G>, DefaultInputTextPluginSetupErr> {
    let rewrite_def_path = config.resource_dir.clone().join("rewrite.def");
    DefaultInputTextPlugin::read_rewrite_lists(rewrite_def_path)
  }
  pub fn read_rewrite_lists_from_reader<R: BufRead>(
    reader: &mut R,
  ) -> Result<DefaultInputTextPlugin<G>, DefaultInputTextPluginSetupErr> {
    let mut key_lengths = HashMap::new();
    let mut ignore_normalize_set = HashSet::new();
    let mut replace_char_map = HashMap::new();
    for (i, line) in reader.lines().enumerate() {
      let line = line?;
      let line = line.trim();
      if line.is_empty() || line.starts_with('#') {
        continue;
      }
      let cols: Vec<&str> = line.split_whitespace().collect();

      // ignored normalize list
      if cols.len() == 1 {
        let key = cols[0].to_string();
        if key.chars().count() != 1 {
          return Err(DefaultInputTextPluginSetupErr::NotCharacterErr(i, key));
        }
        ignore_normalize_set.insert(key);
      // replace char list
      } else if cols.len() == 2 {
        let key = cols[0].to_string();
        if replace_char_map.contains_key(key.as_bytes()) {
          return Err(DefaultInputTextPluginSetupErr::AlreadyDefinedErr(i, key));
        }
        let c = key.chars().nth(0).unwrap();
        if *key_lengths.get(&c).unwrap_or(&0) < key.chars().count() {
          key_lengths.insert(c, key.chars().count());
        }
        replace_char_map.insert(key.as_bytes().to_vec(), cols[1].to_string());
      } else {
        return Err(DefaultInputTextPluginSetupErr::InvalidFormatErr(i));
      }
    }
    Ok(DefaultInputTextPlugin {
      phantom: PhantomData,
      key_lengths,
      replace_char_map,
      ignore_normalize_set,
    })
  }
  pub fn read_rewrite_lists<P: AsRef<Path>>(
    rewrite_def_path: P,
  ) -> Result<DefaultInputTextPlugin<G>, DefaultInputTextPluginSetupErr> {
    let mut reader = BufReader::new(File::open(rewrite_def_path)?);
    DefaultInputTextPlugin::read_rewrite_lists_from_reader(&mut reader)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::Config;
  use crate::dictionary_lib::character_category::CharacterCategory;
  use crate::dictionary_lib::grammar::{GetCharacterCategory, SetCharacterCategory};
  use std::cell::RefCell;
  use std::path::PathBuf;
  use std::rc::Rc;
  use std::str::FromStr;

  fn resources_test_dir() -> PathBuf {
    PathBuf::from_str(file!())
      .unwrap()
      .parent()
      .unwrap()
      .parent()
      .unwrap()
      .join("resources/test")
  }

  const ORIGINAL_TEXT: &str = "ÂＢΓД㈱ｶﾞウ゛⼼Ⅲ";
  const NORMALIZED_TEXT: &str = "âbγд(株)ガヴ⼼ⅲ";
  type CelledMockGrammar = Rc<RefCell<MockGrammar>>;

  struct MockGrammar {
    character_category: Option<CharacterCategory>,
  }
  impl MockGrammar {
    fn new() -> MockGrammar {
      let mut character_category = CharacterCategory::default();
      character_category
        .read_character_definition(resources_test_dir().join("char.def").as_path())
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

  fn setup() -> (
    UTF8InputTextBuilder<CelledMockGrammar>,
    DefaultInputTextPlugin<CelledMockGrammar>,
  ) {
    let builder =
      UTF8InputTextBuilder::new(ORIGINAL_TEXT, Rc::new(RefCell::new(MockGrammar::new())));
    let mut config = Config::empty().unwrap();
    config.resource_dir = PathBuf::from_str(file!())
      .unwrap()
      .parent()
      .unwrap()
      .parent()
      .unwrap()
      .join("resources");
    let plugin = DefaultInputTextPlugin::<CelledMockGrammar>::setup(&config).unwrap();
    (builder, plugin)
  }

  #[test]
  fn test_before_rewrite() {
    let (builder, _) = setup();
    assert_eq!(ORIGINAL_TEXT, builder.get_original_text());
    assert_eq!(ORIGINAL_TEXT, builder.get_text());
    let text = builder.build();
    assert_eq!(ORIGINAL_TEXT, text.get_original_text());
    assert_eq!(ORIGINAL_TEXT, text.get_text());
    let bytes = text.get_byte_text();
    assert_eq!(30, bytes.len());
    let expected = vec![
      0xc3, 0x82, 0xef, 0xbc, 0xa2, 0xce, 0x93, 0xd0, 0x94, 0xe3, 0x88, 0xb1, 0xef, 0xbd, 0xb6,
      0xef, 0xbe, 0x9e, 0xe3, 0x82, 0xa6, 0xe3, 0x82, 0x9b, 0xe2, 0xbc, 0xbc, 0xe2, 0x85, 0xa2,
    ];
    assert_eq!(&expected, bytes);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(0, text.get_original_index(1));
    assert_eq!(1, text.get_original_index(2));
    assert_eq!(1, text.get_original_index(4));
    assert_eq!(3, text.get_original_index(8));
    assert_eq!(5, text.get_original_index(12));
    assert_eq!(9, text.get_original_index(24));
    assert_eq!(9, text.get_original_index(26));
  }

  #[test]
  fn test_after_write() {
    let (mut builder, plugin) = setup();
    assert_eq!(ORIGINAL_TEXT, builder.get_original_text());
    assert_eq!(ORIGINAL_TEXT, builder.get_text());
    plugin.rewrite(&mut builder).unwrap();
    let text = builder.build();
    assert_eq!(ORIGINAL_TEXT, text.get_original_text());
    assert_eq!(NORMALIZED_TEXT, text.get_text());
    let bytes = text.get_byte_text();
    assert_eq!(24, bytes.len());
    let expected = vec![
      0xc3, 0xa2, 0x62, 0xce, 0xb3, 0xd0, 0xb4, 0x28, 0xe6, 0xa0, 0xaa, 0x29, 0xe3, 0x82, 0xac,
      0xe3, 0x83, 0xb4, 0xe2, 0xbc, 0xbc, 0xe2, 0x85, 0xb2,
    ];
    assert_eq!(&expected, bytes);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(0, text.get_original_index(1));
    assert_eq!(1, text.get_original_index(2));
    assert_eq!(2, text.get_original_index(3));
    assert_eq!(4, text.get_original_index(7));
    assert_eq!(4, text.get_original_index(11));
    assert_eq!(7, text.get_original_index(15));
    assert_eq!(7, text.get_original_index(17));
  }

  #[test]
  fn test_invalid_format_ignorelist() {
    let rewrite_def_path_buf = resources_test_dir().join("rewrite_error_ignorelist.def");
    let mut reader = BufReader::new(File::open(rewrite_def_path_buf).unwrap());
    let err =
      DefaultInputTextPlugin::<Rc<RefCell<Grammar>>>::read_rewrite_lists_from_reader(&mut reader)
        .err()
        .unwrap();
    assert_eq!("12 is not character at line 1", format!("{}", err));
  }

  #[test]
  fn test_invalid_format_replacelist() {
    let rewrite_def_path_buf = resources_test_dir().join("rewrite_error_replacelist.def");
    let mut reader = BufReader::new(File::open(rewrite_def_path_buf).unwrap());
    let err =
      DefaultInputTextPlugin::<Rc<RefCell<Grammar>>>::read_rewrite_lists_from_reader(&mut reader)
        .err()
        .unwrap();
    assert_eq!("invalid format at line 1", format!("{}", err));
  }

  #[test]
  fn test_duplicated_lines_replacelist() {
    let rewrite_def_path_buf = resources_test_dir().join("rewrite_error_dup.def");
    let mut reader = BufReader::new(File::open(rewrite_def_path_buf).unwrap());
    let err =
      DefaultInputTextPlugin::<Rc<RefCell<Grammar>>>::read_rewrite_lists_from_reader(&mut reader)
        .err()
        .unwrap();
    assert_eq!("12 is already defined at line 2", format!("{}", err));
  }
}
