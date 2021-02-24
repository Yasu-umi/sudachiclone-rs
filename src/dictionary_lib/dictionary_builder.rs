use std::char::from_u32;
use std::collections::HashMap;
use std::io::{BufRead, Cursor, Error as IOError, Seek, SeekFrom, Write};
use std::num::ParseIntError;
use std::str::FromStr;

use log::{error, info, warn};
use regex::{Captures, Error as RegexError, Regex};
use thiserror::Error;

use super::io::{CurrentPosition, LittleEndianWrite, Pipe};
use super::lexicon::LexiconErr;
use super::word_info::WordInfo;
use crate::darts::DoubleArrayTrie;

const BYTE_MAX_VALUE: usize = 127;
// const MAX_LENGTH: u64 = 255;
const COLS_NUM: usize = 18;
// const BUFFER_SIZE: u64 = 1024 * 1024;
// const PATTERN_UNICODE_LITERAL: Regex = Regex::new(r"\\u([0-9a-fA-F]{4}|{[0-9a-fA-F]+})").unwrap();
const ARRAY_MAX_LENGTH: usize = 127; // max value of byte in Java
const STRING_MAX_LENGTH: usize = 32767; // max value of short in Java

pub struct WordEntry {
  headword: Option<String>,
  parameters: [i16; 3],
  word_info: WordInfo,
  aunit_split_string: String,
  bunit_split_string: String,
  cunit_split_string: String,
}

struct PosTable {
  table: Vec<String>,
}

impl PosTable {
  fn new() -> PosTable {
    PosTable { table: vec![] }
  }
  fn get_id(&self, text: &str) -> Result<usize, DictionaryBuilderErr> {
    self
      .table
      .iter()
      .position(|t| t == text)
      .ok_or_else(|| DictionaryBuilderErr::InvalidFormatErr)
  }
  fn mut_get_id(&mut self, text: &str) -> usize {
    match self.table.iter().position(|t| t == text) {
      Some(id) => id,
      None => {
        self.table.push(text.to_string());
        self.table.len() - 1
      }
    }
  }
  pub fn get_list(&self) -> &Vec<String> {
    &self.table
  }
}

#[derive(Debug, Error)]
pub enum DictionaryBuilderErr {
  #[error("invalid format")]
  InvalidFormatErr,
  #[error("invalid word id")]
  InvalidWordIdErr,
  #[error("{0}")]
  IOError(#[from] IOError),
  #[error("{0}")]
  ParseIntError(#[from] ParseIntError),
  #[error("{0}")]
  CSVError(#[from] csv::Error),
  #[error("{0}")]
  RegexError(#[from] RegexError),
  #[error("{0}")]
  LexiconErr(#[from] LexiconErr),
}

pub struct DictionaryBuilder {
  trie_keys: HashMap<String, Vec<usize>>,
  pub entries: Vec<WordEntry>,
  _is_user_dictionary: bool,
  pos_table: PosTable,
}

impl Default for DictionaryBuilder {
  fn default() -> DictionaryBuilder {
    DictionaryBuilder {
      trie_keys: HashMap::new(),
      entries: vec![],
      _is_user_dictionary: false,
      pos_table: PosTable::new(),
    }
  }
}

impl DictionaryBuilder {
  pub fn build<R: BufRead, W: Write + Seek>(
    &mut self,
    lexicon_paths: &[&str],
    matrix_reader: Option<&mut R>,
    writer: &mut W,
  ) -> Result<(), DictionaryBuilderErr> {
    info!("reading the source file...");
    for path in lexicon_paths {
      self.build_lexicons(path)?;
    }
    info!("{} words", self.entries.len());
    self.write_grammar(matrix_reader, writer)?;
    self.write_lexicon(writer)?;
    Ok(())
  }
  pub fn build_lexicons(&mut self, path: &str) -> Result<(), DictionaryBuilderErr> {
    for (i, record) in csv::ReaderBuilder::new()
      .has_headers(false)
      .from_path(path)?
      .into_records()
      .enumerate()
    {
      match self.build_lexicon(record) {
        Ok(_) => (),
        Err(e) => {
          error!("{} as line {} in {}", e, i + 1, path);
          return Err(e);
        }
      }
    }
    Ok(())
  }
  fn build_lexicon(
    &mut self,
    record: Result<csv::StringRecord, csv::Error>,
  ) -> Result<(), DictionaryBuilderErr> {
    let row = match record {
      Ok(r) => r.into_iter().map(|c| c.to_string()).collect(),
      Err(e) => return Err(DictionaryBuilderErr::CSVError(e)),
    };

    let entry = match self.parse_line(row) {
      Ok(e) => e,
      Err(e) => return Err(e),
    };
    if let Some(headword) = entry.headword.as_ref() {
      self.add_to_trie(headword, self.entries.len());
    }
    self.entries.push(entry);
    Ok(())
  }
  fn parse_line(&mut self, cols: Vec<String>) -> Result<WordEntry, DictionaryBuilderErr> {
    if cols.len() != COLS_NUM {
      return Err(DictionaryBuilderErr::InvalidFormatErr);
    }
    let cols: Vec<String> = cols
      .iter()
      .map(|col| DictionaryBuilder::decode(col))
      .collect();
    if !DictionaryBuilder::is_length_valid(&cols) {
      return Err(DictionaryBuilderErr::InvalidFormatErr);
    }
    if cols[0].is_empty() {
      return Err(DictionaryBuilderErr::InvalidFormatErr);
    }
    let headword = if cols[1] != "-1" {
      Some(cols[0].clone())
    } else {
      None
    };
    let parameters = [
      cols[1].parse::<i16>()?,
      cols[2].parse::<i16>()?,
      cols[3].parse::<i16>()?,
    ];
    let strs: Vec<&str> = cols.iter().map(|c| c.as_str()).collect();
    let pos_id = self.mut_get_pos_id(&strs[5..11]);
    // 通らない
    // if pos_id < 0 {
    // }
    let aunit_split_string = DictionaryBuilder::check_splitinfo_format(&cols[15])?.to_string();
    let bunit_split_string = DictionaryBuilder::check_splitinfo_format(&cols[16])?.to_string();
    let cunit_split_string = DictionaryBuilder::check_splitinfo_format(&cols[17])?.to_string();

    let dictionary_form_word_id = if cols[13] == "*" {
      -1
    } else {
      cols[13].parse::<i32>()?
    };
    Ok(WordEntry {
      headword,
      parameters,
      word_info: WordInfo {
        surface: cols[4].clone(),
        head_word_length: cols[0].len(),
        pos_id: pos_id as i16,
        normalized_form: cols[12].clone(),
        dictionary_form_word_id,
        dictionary_form: String::from(""),
        reading_form: cols[11].clone(),
        a_unit_split: vec![],
        b_unit_split: vec![],
        word_structure: vec![],
      },
      aunit_split_string,
      bunit_split_string,
      cunit_split_string,
    })
  }
  fn add_to_trie(&mut self, headword: &str, word_id: usize) {
    match self.trie_keys.get_mut(headword) {
      Some(v) => v.push(word_id),
      None => {
        self.trie_keys.insert(headword.to_string(), vec![word_id]);
      }
    };
  }
  pub fn write_grammar<R: BufRead, W: Write + Seek>(
    &mut self,
    matrix_reader: Option<&mut R>,
    writer: &mut W,
  ) -> Result<(), DictionaryBuilderErr> {
    info!("writing the POS table...");
    let start = writer.position()?;
    DictionaryBuilder::convert_pos_table(writer, &self.pos_table.get_list().clone())?;
    let end = writer.position()?;
    DictionaryBuilder::logging_size(end - start);

    info!("writing the connection matrix...");
    match matrix_reader {
      Some(reader) => self.write_matrix(reader, writer)?,
      None => self.write_empty_matrix(writer)?,
    }
    Ok(())
  }
  fn write_matrix<R: BufRead, W: Write>(
    &mut self,
    matrix_reader: &mut R,
    writer: &mut W,
  ) -> Result<(), DictionaryBuilderErr> {
    let mut header = String::new();
    matrix_reader.read_line(&mut header)?;
    let header = header.trim();
    if Regex::new(r"^\s*$")?.is_match(&header) {
      return Err(DictionaryBuilderErr::InvalidFormatErr);
    }

    let mut lr = header.split_whitespace();
    let lsize = i16::from_str(lr.next().ok_or(DictionaryBuilderErr::InvalidFormatErr)?)?;
    let rsize = i16::from_str(lr.next().ok_or(DictionaryBuilderErr::InvalidFormatErr)?)?;
    writer.write_i16(lsize)?;
    writer.write_i16(rsize)?;

    let mut matrix = Cursor::new(vec![]);

    for (i, l) in matrix_reader.lines().enumerate() {
      match l {
        Ok(line) => {
          if Regex::new(r"^\s*$").unwrap().is_match(&line) || line.contains('#') {
            continue;
          }
          let cols: Vec<&str> = line.split_whitespace().collect();
          if cols.len() < 3 {
            warn!("invalid format at line {}", i);
            continue;
          }
          let l = u64::from_str(cols[0])?;
          let r = u64::from_str(cols[1])?;
          let cost = i16::from_str(cols[2])?;
          let pos = matrix.position();
          matrix.set_position(2 * (l + (lsize as u64) * r));
          matrix.write_i16(cost)?;
          matrix.set_position(pos);
        }
        Err(e) => return Err(DictionaryBuilderErr::IOError(e)),
      }
    }
    matrix.pipe_all(writer)?;
    DictionaryBuilder::logging_size(matrix.get_ref().len() + 4);
    Ok(())
  }
  fn write_empty_matrix<W: Write + Seek>(
    &mut self,
    writer: &mut W,
  ) -> Result<(), DictionaryBuilderErr> {
    writer.write_i16(0)?;
    writer.write_i16(0)?;
    DictionaryBuilder::logging_size(writer.position()?);
    Ok(())
  }
  fn convert_pos_table<W: Write>(
    writer: &mut W,
    table: &[String],
  ) -> Result<(), DictionaryBuilderErr> {
    writer.write_i16(table.len() as i16)?;
    for pos in table {
      for text in pos.split(',') {
        DictionaryBuilder::write_string_to_writer(writer, text)?;
      }
    }
    Ok(())
  }
  pub fn write_lexicon<W: Write + Seek>(
    &mut self,
    writer: &mut W,
  ) -> Result<(), DictionaryBuilderErr> {
    let mut trie = DoubleArrayTrie::default();
    let mut cursor = Cursor::new(vec![]);
    let mut items: Vec<_> = self.trie_keys.iter().collect();
    let mut keys = Vec::with_capacity(items.len());
    let mut vals = Vec::with_capacity(items.len());
    items.sort_by(|(key1, _), (key2, _)| key1.cmp(key2));
    for (key, word_ids) in items {
      keys.push(key.as_bytes());
      vals.push(cursor.position() as u32);
      cursor.write_u8(word_ids.len() as u8)?;
      for wid in word_ids {
        cursor.write_u32(*wid as u32)?;
      }
    }

    info!("building the trie...");
    trie.build(&keys, &vals);
    info!("done");
    info!("writing the trie...");
    let size = trie.size();
    writer.write_u32(size as u32)?;

    let mut buf = Vec::with_capacity(size * 4);
    for u in trie.get_array() {
      buf.extend(&u.to_le_bytes());
    }
    writer.write_all(&buf)?;
    DictionaryBuilder::logging_size(size * 4 + 4);

    info!("writing the word-ID table...");
    writer.write_u32(cursor.position() as u32)?;

    cursor.set_position(0);
    cursor.pipe_all(writer)?;
    DictionaryBuilder::logging_size((cursor.position() + 4) as usize);

    info!("writing the word parameters...");
    writer.write_u32(self.entries.len() as u32)?;
    for entry in self.entries.iter() {
      writer.write_i16(entry.parameters[0])?;
      writer.write_i16(entry.parameters[1])?;
      writer.write_i16(entry.parameters[2])?;
    }
    DictionaryBuilder::logging_size(self.entries.len() * 6 + 4);
    self.write_word_info(writer)?;

    Ok(())
  }
  fn write_word_info<W: Write + Seek>(
    &mut self,
    writer: &mut W,
  ) -> Result<(), DictionaryBuilderErr> {
    let mark = writer.position()?;
    let base = mark * 4 + self.entries.len();
    writer.seek(SeekFrom::Start(base as u64))?;

    let mut offsets = Cursor::new(Vec::with_capacity(self.entries.len() * 4));
    info!("writing the word_infos...");

    let entries: &Vec<WordEntry> = self.entries.as_ref();
    for entry in entries.iter() {
      let word_info = &entry.word_info;
      offsets.write_u32(writer.position()? as u32)?;

      DictionaryBuilder::write_string_to_writer(writer, &word_info.surface)?;

      DictionaryBuilder::write_string_length_to_writer(writer, word_info.head_word_length)?;

      writer.write_i16(word_info.pos_id as i16)?;

      let normalized_form = if word_info.normalized_form == word_info.surface {
        ""
      } else {
        &word_info.normalized_form
      };
      DictionaryBuilder::write_string_to_writer(writer, normalized_form)?;

      writer.write_u32(word_info.dictionary_form_word_id as u32)?;

      let reading_form = if word_info.reading_form == word_info.surface {
        ""
      } else {
        &word_info.reading_form
      };
      DictionaryBuilder::write_string_to_writer(writer, reading_form)?;

      let a_unit_splitinfo = self.parse_splitinfo(&entry.aunit_split_string)?;
      DictionaryBuilder::write_i32_vec_to_writer(writer, a_unit_splitinfo)?;

      let bunit_splitinfo = self.parse_splitinfo(&entry.bunit_split_string)?;
      DictionaryBuilder::write_i32_vec_to_writer(writer, bunit_splitinfo)?;

      let cunit_splitinfo = self.parse_splitinfo(&entry.cunit_split_string)?;
      DictionaryBuilder::write_i32_vec_to_writer(writer, cunit_splitinfo)?;
    }
    DictionaryBuilder::logging_size(writer.position()? - base);
    info!("writing word_info offsets...");

    writer.seek(SeekFrom::Start(mark as u64))?;
    offsets.set_position(0);
    offsets.pipe_all(writer)?;

    DictionaryBuilder::logging_size(offsets.position() as usize);
    Ok(())
  }
  fn decode(text: &str) -> String {
    let re = Regex::new(r"\\u([0-9a-fA-F]{4}|\{[0-9a-fA-F]+\})").unwrap();
    re.replace_all(text, |caps: &Captures| match caps.get(0) {
      Some(uni_text) => {
        let uni_text = uni_text.as_str().replace("{", "").replace("}", "");
        from_u32(u32::from_str_radix(&uni_text[2..uni_text.len()], 16).unwrap())
          .unwrap()
          .to_string()
      }
      None => String::from(""),
    })
    .to_string()
  }
  pub fn parse_splitinfo(&self, info: &str) -> Result<Vec<u32>, DictionaryBuilderErr> {
    parse_splitinfo(self, info)
  }
  fn is_length_valid(cols: &[String]) -> bool {
    let head_length = cols[0].chars().count();
    head_length <= STRING_MAX_LENGTH
      && cols[4].chars().count() <= STRING_MAX_LENGTH
      && cols[11].chars().count() <= STRING_MAX_LENGTH
      && cols[12].chars().count() <= STRING_MAX_LENGTH
  }
  fn is_id(text: &str) -> bool {
    Regex::new(r"U?\d+").unwrap().is_match(text)
  }
  pub fn check_word_id(&self, word_id: u32) -> Result<(), DictionaryBuilderErr> {
    if
    /* word_id < 0 || */
    word_id >= self.entries.len() as u32 {
      return Err(DictionaryBuilderErr::InvalidWordIdErr);
    }
    Ok(())
  }
  fn mut_get_pos_id(&mut self, strs: &[&str]) -> u16 {
    self.pos_table.mut_get_id(&strs.join(",")) as u16
  }
  fn check_splitinfo_format(text: &str) -> Result<&str, DictionaryBuilderErr> {
    if text.split('/').count() > ARRAY_MAX_LENGTH {
      return Err(DictionaryBuilderErr::InvalidFormatErr);
    }
    Ok(text)
  }
  fn write_string_to_writer<W: Write>(
    writer: &mut W,
    text: &str,
  ) -> Result<(), DictionaryBuilderErr> {
    let mut len = 0 as u32;
    for c in text.chars().map(|c| c as u32) {
      len += if 0x10000 <= c && c <= 0x0010_FFFF {
        2
      } else {
        1
      };
    }
    DictionaryBuilder::write_string_length_to_writer(writer, len as usize)?;
    writer.write_utf16_str(text)?;
    Ok(())
  }
  fn write_string_length_to_writer<W: Write>(
    writer: &mut W,
    len: usize,
  ) -> Result<(), DictionaryBuilderErr> {
    if len <= BYTE_MAX_VALUE {
      writer.write_u8(len as u8)?;
    } else {
      writer.write_u8(((len >> 8) | 0x80) as u8)?;
      writer.write_u8((len & 0xFF) as u8)?;
    }
    Ok(())
  }
  fn write_i32_vec_to_writer<W: Write>(
    writer: &mut W,
    array: Vec<u32>,
  ) -> Result<(), DictionaryBuilderErr> {
    writer.write_u8(array.len() as u8)?;
    for i in array {
      writer.write_u32(i)?;
    }
    Ok(())
  }
  fn logging_size(size: usize) {
    info!("{} bytes", size);
  }
}

pub trait IdParser {
  fn is_user_dictionary(&self) -> bool;
  fn check_word_id(&self, word_id: u32) -> Result<(), DictionaryBuilderErr>;
}

impl IdParser for DictionaryBuilder {
  fn is_user_dictionary(&self) -> bool {
    self._is_user_dictionary
  }
  fn check_word_id(&self, word_id: u32) -> Result<(), DictionaryBuilderErr> {
    if
    /* word_id < 0 || */
    word_id >= self.entries.len() as u32 {
      return Err(DictionaryBuilderErr::InvalidWordIdErr);
    }
    Ok(())
  }
}

pub fn parse_id<T: IdParser>(this: &T, text: &str) -> Result<u32, DictionaryBuilderErr> {
  let id = if text.starts_with('U') {
    let mut id = u32::from_str(&text[1..])?;
    if this.is_user_dictionary() {
      id |= 1 << 28;
    }
    id
  } else {
    u32::from_str(text)?
  };
  this.check_word_id(id)?;
  Ok(id)
}

pub trait WordIdToIdConverter {
  fn get_pos_id(&self, strs: &[&str]) -> Result<u16, DictionaryBuilderErr>;
  fn get_word_id(
    &self,
    headword: &str,
    pos_id: u16,
    reading_form: &str,
  ) -> Result<u32, DictionaryBuilderErr>;
}

impl WordIdToIdConverter for DictionaryBuilder {
  fn get_pos_id(&self, strs: &[&str]) -> Result<u16, DictionaryBuilderErr> {
    Ok(self.pos_table.get_id(&strs.join(","))? as u16)
  }
  fn get_word_id(
    &self,
    headword: &str,
    pos_id: u16,
    reading_form: &str,
  ) -> Result<u32, DictionaryBuilderErr> {
    for i in 0..self.entries.len() {
      let info = &self.entries[i].word_info;
      if info.surface == headword
        && info.pos_id == pos_id as i16
        && info.reading_form == reading_form
      {
        return Ok(i as u32);
      }
    }
    Err(DictionaryBuilderErr::InvalidFormatErr)
  }
}

pub fn word_to_id<T: WordIdToIdConverter>(
  this: &T,
  text: &str,
) -> Result<u32, DictionaryBuilderErr> {
  let cols: Vec<&str> = text.split(',').collect();
  if cols.len() < 8 {
    return Err(DictionaryBuilderErr::InvalidFormatErr);
  }
  let headword = DictionaryBuilder::decode(cols[0]);
  let pos_id = this.get_pos_id(&cols[1..7])?;
  // if pos_id < 0 {
  //   return Err(Box::new(InvalidFormatErr::new()));
  // }
  let reading_form = DictionaryBuilder::decode(cols[7]);
  this.get_word_id(&headword, pos_id, &reading_form)
}

pub trait SplitInfoParser {
  fn parse_id(&self, text: &str) -> Result<u32, DictionaryBuilderErr>;
  fn word_to_id(&self, text: &str) -> Result<u32, DictionaryBuilderErr>;
}

impl SplitInfoParser for DictionaryBuilder {
  fn parse_id(&self, text: &str) -> Result<u32, DictionaryBuilderErr> {
    parse_id(self, text)
  }
  fn word_to_id(&self, text: &str) -> Result<u32, DictionaryBuilderErr> {
    word_to_id(self, text)
  }
}

pub fn parse_splitinfo<T: SplitInfoParser>(
  this: &T,
  info: &str,
) -> Result<Vec<u32>, DictionaryBuilderErr> {
  if info == "*" {
    return Ok(vec![]);
  }
  let words: Vec<&str> = info.split('/').collect();
  if words.len() > ARRAY_MAX_LENGTH {
    return Err(DictionaryBuilderErr::InvalidFormatErr);
  }
  let mut ids = vec![];
  for word in words {
    if DictionaryBuilder::is_id(word) {
      ids.push(this.parse_id(word)?);
    } else {
      ids.push(this.word_to_id(word)?);
    }
  }
  Ok(ids)
}

pub fn build_empty_entry() -> WordEntry {
  WordEntry {
    headword: None,
    parameters: [0, 0, 0],
    word_info: WordInfo {
      surface: String::from(""),
      head_word_length: 0,
      pos_id: 0,
      normalized_form: String::from(""),
      dictionary_form_word_id: 0,
      dictionary_form: String::from(""),
      reading_form: String::from(""),
      a_unit_split: vec![],
      b_unit_split: vec![],
      word_structure: vec![],
    },
    aunit_split_string: String::from(""),
    bunit_split_string: String::from(""),
    cunit_split_string: String::from(""),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[cfg(not(target_arch = "wasm32"))]
  use crate::dictionary_lib::dictionary_header::DictionaryHeader;
  #[cfg(not(target_arch = "wasm32"))]
  use crate::dictionary_lib::double_array_lexicon::DoubleArrayLexicon;
  #[cfg(not(target_arch = "wasm32"))]
  use crate::dictionary_lib::grammar::Grammar;
  #[cfg(not(target_arch = "wasm32"))]
  use crate::dictionary_lib::lexicon::{Lexicon, Size};
  #[cfg(not(target_arch = "wasm32"))]
  use crate::dictionary_lib::lexicon_set::LexiconSet;
  #[cfg(not(target_arch = "wasm32"))]
  use crate::dictionary_lib::system_dictionary_version::SYSTEM_DICT_VERSION;

  use encoding_rs::UTF_16LE;

  #[cfg(not(target_arch = "wasm32"))]
  use std::env::temp_dir;
  #[cfg(not(target_arch = "wasm32"))]
  use std::fs::{create_dir, remove_dir_all, File};
  use std::io::Read;
  #[cfg(not(target_arch = "wasm32"))]
  use std::path::{Path, PathBuf};

  #[test]
  fn test_parse_line() {
    let mut builder = DictionaryBuilder::default();
    let entry = builder
      .parse_line(
        "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*"
          .split(',')
          .map(|s| s.to_string())
          .collect(),
      )
      .unwrap();
    assert_eq!("京都", entry.headword.unwrap());
    assert_eq!([6, 6, 5293], entry.parameters);
    assert_eq!(0, entry.word_info.pos_id);
    assert_eq!("*", entry.aunit_split_string);
    assert_eq!("*", entry.bunit_split_string);
    let entry = builder
      .parse_line(
        "京都,-1,-1,0,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*"
          .split(',')
          .map(|s| s.to_string())
          .collect(),
      )
      .unwrap();
    assert_eq!(None, entry.headword);
    assert_eq!(0, entry.word_info.pos_id);
  }

  #[test]
  fn test_parse_line_invalid_columns() {
    let mut builder = DictionaryBuilder::default();
    assert!(builder
      .parse_line(
        "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*"
          .split(',')
          .map(|s| s.to_string())
          .collect(),
      )
      .is_err());
  }

  #[test]
  fn test_parse_line_empty_headword() {
    let mut builder = DictionaryBuilder::default();
    assert!(builder
      .parse_line(
        ",6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*"
          .split(',')
          .map(|s| s.to_string())
          .collect(),
      )
      .is_err());
  }

  #[test]
  fn test_parse_line_toolong_headword() {
    let mut builder = DictionaryBuilder::default();
    assert!(builder
      .parse_line(
        format!(
          "{},6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*",
          (0..=32767).map(|_| "a").collect::<Vec<&str>>().join("")
        )
        .split(',')
        .map(|s| s.to_string())
        .collect(),
      )
      .is_err());
  }

  #[test]
  fn test_parse_line_toomany_split() {
    let mut builder = DictionaryBuilder::default();
    assert!(builder
      .parse_line(
        "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,B,0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0/0/1/2/3/4/5/6/7/8/9/0,*,*"
        .split(',')
        .map(|s| s.to_string())
        .collect(),
      )
      .is_err());
  }

  #[test]
  fn test_parse_line_same_readingform() {
    let mut builder = DictionaryBuilder::default();
    let entry = builder
      .parse_line(
        "〒,6,6,5293,〒,名詞,普通名詞,一般,*,*,*,〒,〒,*,A,*,*,*"
          .split(',')
          .map(|s| s.to_string())
          .collect(),
      )
      .unwrap();
    assert_eq!("〒", entry.word_info.reading_form);
  }

  #[test]
  fn test_add_to_trie() {
    let mut builder = DictionaryBuilder::default();
    builder.add_to_trie("abc", 0);
    builder.add_to_trie("abc", 1);
    builder.add_to_trie("abcd", 2);
    assert!(builder.trie_keys["abc"].contains(&0));
    assert!(builder.trie_keys["abc"].contains(&1));
    assert!(builder.trie_keys["abcd"].contains(&2));
  }

  #[test]
  fn test_convert_pos_table() {
    let mut writer = Cursor::new(vec![]);
    DictionaryBuilder::convert_pos_table(
      &mut writer,
      &[String::from("a,b,c,d,e,f"), String::from("g,h,i,j,k,l")],
    )
    .unwrap();
    assert_eq!(2 + 3 * 12, writer.get_ref().len());
  }

  #[test]
  fn test_write_matrix() {
    let mut builder = DictionaryBuilder::default();
    let mut writer = Cursor::new(vec![]);
    let mut matrix_reader =
      Cursor::new("2 3\n0 0 0\n0 1 1\n0 2 2\n\n1 0 3\n1 1 4\n1 2 5\n".as_bytes());
    builder
      .write_matrix(&mut matrix_reader, &mut writer)
      .unwrap();

    writer.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = [0, 0];
    writer.read_exact(&mut buf).unwrap();
    assert_eq!(2, i16::from_le_bytes(buf));

    let mut buf = [0, 0];
    writer.read_exact(&mut buf).unwrap();
    assert_eq!(3, i16::from_le_bytes(buf));

    writer.seek(SeekFrom::Start(4)).unwrap();
    let mut buf = [0, 0];
    writer.read_exact(&mut buf).unwrap();
    assert_eq!(0, u16::from_le_bytes(buf));

    writer.seek(SeekFrom::Start(4 + 6)).unwrap();
    let mut buf = [0, 0];
    writer.read_exact(&mut buf).unwrap();
    assert_eq!(4, u16::from_le_bytes(buf));
  }

  #[test]
  fn test_decode() {
    assert_eq!("a,c", DictionaryBuilder::decode("a\\u002cc"));
    assert_eq!("a,c", DictionaryBuilder::decode("a\\u{002c}c"));
    assert_eq!("a𠮟c", DictionaryBuilder::decode("a\\u{20b9f}c"));
  }

  #[test]
  fn test_parse_splitinfo() {
    let mut builder = DictionaryBuilder::default();
    builder.entries.extend(vec![
      build_empty_entry(),
      build_empty_entry(),
      build_empty_entry(),
      build_empty_entry(),
    ]);
    assert_eq!(
      vec![] as Vec<u32>,
      builder.parse_splitinfo(&String::from("*")).unwrap(),
    );
    assert_eq!(
      vec![1, 2, 3],
      builder.parse_splitinfo(&String::from("1/2/3")).unwrap(),
    );
    assert_eq!(
      2,
      builder.parse_splitinfo(&String::from("1/U2/3")).unwrap()[1],
    );
  }

  #[test]
  fn test_parse_splitinfo_invalid_word_id() {
    let builder = DictionaryBuilder::default();
    assert_eq!(
      "invalid word id",
      format!(
        "{}",
        builder
          .parse_splitinfo(&String::from("1/2/3"))
          .err()
          .unwrap()
      )
    );
  }

  #[test]
  fn test_write_string_to_writer() {
    let mut cursor = Cursor::new(vec![]);

    let position = cursor.position();
    DictionaryBuilder::write_string_to_writer(&mut cursor, "").unwrap();
    assert_eq!(0, cursor.get_ref()[0]);
    assert_eq!(position + 1, cursor.position());

    let position = cursor.position() as usize;
    DictionaryBuilder::write_string_to_writer(&mut cursor, "あ𠮟").unwrap();
    assert_eq!(3, cursor.get_ref()[position as usize]);
    assert_eq!(
      "あ",
      UTF_16LE
        .decode(&cursor.get_ref()[position + 1..position + 3])
        .0
    );
    assert_eq!(
      55362,
      u16::from_le_bytes([
        cursor.get_ref()[position + 3],
        cursor.get_ref()[position + 4],
      ])
    );
    assert_eq!(
      57247,
      u16::from_le_bytes([
        cursor.get_ref()[position + 5],
        cursor.get_ref()[position + 6],
      ])
    );

    let position = cursor.position() as usize;
    let long_str = "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
    let len = long_str.chars().count();
    DictionaryBuilder::write_string_to_writer(&mut cursor, long_str).unwrap();
    assert_eq!((len >> 8) | 0x80, cursor.get_ref()[position] as usize);
    assert_eq!(len & 0xff, cursor.get_ref()[position + 1] as usize);
    assert_eq!(position + 2 + 2 * len, cursor.position() as usize);
  }

  #[test]
  fn test_write_i32_vec_to_writer() {
    let mut cursor = Cursor::new(vec![]);

    let position = cursor.position() as usize;
    DictionaryBuilder::write_i32_vec_to_writer(&mut cursor, vec![]).unwrap();
    assert_eq!(0, cursor.get_ref()[position]);
    DictionaryBuilder::write_i32_vec_to_writer(&mut cursor, vec![1, 2, 3]).unwrap();
    assert_eq!(3, cursor.get_ref()[position + 1]);
    assert_eq!(
      1,
      i32::from_le_bytes([
        cursor.get_ref()[position + 2],
        cursor.get_ref()[position + 3],
        cursor.get_ref()[position + 4],
        cursor.get_ref()[position + 5],
      ])
    );
    assert_eq!(
      2,
      i32::from_le_bytes([
        cursor.get_ref()[position + 6],
        cursor.get_ref()[position + 7],
        cursor.get_ref()[position + 8],
        cursor.get_ref()[position + 9],
      ])
    );
    assert_eq!(
      3,
      i32::from_le_bytes([
        cursor.get_ref()[position + 10],
        cursor.get_ref()[position + 11],
        cursor.get_ref()[position + 12],
        cursor.get_ref()[position + 13],
      ])
    );
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn setup_input() -> (PathBuf, PathBuf) {
    let dir = temp_dir().join("test");
    if Path::exists(&dir) {
      remove_dir_all(&dir).unwrap();
    }
    create_dir(&dir).unwrap();
    let input_path = dir.join("input.txt");
    let mut f = File::create(&input_path).unwrap();
    f.write_all("東京都,0,0,0,東京都,名詞,固有名詞,地名,一般,*,*,ヒガシキョウト,東京都,*,B,\"東,名詞,普通名詞,一般,*,*,*,ヒガシ/2\",*,1/2\n".as_bytes())
      .unwrap();
    f.write_all("東,-1,-1,0,東,名詞,普通名詞,一般,*,*,*,ヒガシ,ひがし,*,A,*,*,*\n".as_bytes())
      .unwrap();
    f.write_all("京都,0,0,0,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*\n".as_bytes())
      .unwrap();

    (dir, input_path)
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn read_system_dictionary<R: BufRead + Seek>(
    reader: &mut R,
  ) -> (DictionaryHeader, Grammar, LexiconSet) {
    let header = DictionaryHeader::from_reader(reader).unwrap();

    let grammar = Grammar::from_reader(reader).unwrap();

    let lexicon_set = LexiconSet::new(DoubleArrayLexicon::from_reader(reader).unwrap());

    (header, grammar, lexicon_set)
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test]
  fn test_build() {
    let (_dir, input_path) = setup_input();

    let mut stream = Cursor::new(vec![]);
    let mut matrix_reader = Cursor::new(b"1 1\n0 0 200\n");
    let header = DictionaryHeader::new(
      SYSTEM_DICT_VERSION,
      DictionaryHeader::get_time(),
      String::from("test"),
    );
    stream.write_all(&header.to_bytes().unwrap()).unwrap();
    let mut builder = DictionaryBuilder::default();
    builder
      .build(
        &[input_path.to_str().unwrap()],
        Some(&mut matrix_reader),
        &mut stream,
      )
      .unwrap();
    stream.seek(SeekFrom::Start(0)).unwrap();

    let (header, grammar, lexicon_set) = read_system_dictionary(&mut stream);
    let lexicon = &lexicon_set.first();

    // header
    assert_eq!(SYSTEM_DICT_VERSION, header.version);
    assert_eq!("test", header.description);

    // grammar
    assert_eq!(2, grammar.get_part_of_speech_size());
    let part_of_speech_string_0: Vec<String> = ["名詞", "固有名詞", "地名", "一般", "*", "*"]
      .iter()
      .map(|s| (*s).to_string())
      .collect();
    assert_eq!(
      &part_of_speech_string_0,
      grammar.get_part_of_speech_string(0)
    );
    let part_of_speech_string_1: Vec<String> = ["名詞", "普通名詞", "一般", "*", "*", "*"]
      .iter()
      .map(|s| (*s).to_string())
      .collect();
    assert_eq!(
      &part_of_speech_string_1,
      grammar.get_part_of_speech_string(1)
    );
    assert_eq!(200, grammar.get_connect_cost(0, 0));

    // lexicon
    assert_eq!(3, lexicon.size());
    assert_eq!(0, lexicon.get_cost(0));
    let word_info = lexicon.get_word_info(0);
    assert_eq!("東京都", word_info.surface);
    assert_eq!("東京都", word_info.normalized_form);
    assert_eq!(-1, word_info.dictionary_form_word_id);
    assert_eq!("ヒガシキョウト", word_info.reading_form);
    assert_eq!(0, word_info.pos_id);
    assert_eq!(vec![1, 2], word_info.a_unit_split);
    assert_eq!(vec![0i32; 0], word_info.b_unit_split);
  }
}
