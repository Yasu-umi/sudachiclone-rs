use std::fs::File;
use std::io::{BufRead, BufReader, Error as IOError, Seek};
use std::path::Path;

use thiserror::Error;

use super::dictionary_header::{DictionaryHeader, DictionaryHeaderErr};
use super::double_array_lexicon::DoubleArrayLexicon;
use super::grammar::Grammar;
use super::lexicon::LexiconErr;
use super::system_dictionary_version::{
  SYSTEM_DICT_VERSION_1, SYSTEM_DICT_VERSION_2, USER_DICT_VERSION_1, USER_DICT_VERSION_2, USER_DICT_VERSION_3
};

#[derive(Error, Debug)]
pub enum ReadDictionaryErr {
  #[error("invalid dictionary version")]
  InvalidDictionaryVersionErr,
  #[error("invalid system dictionary")]
  InvalidSystemDictionaryErr,
  #[error("invalid user dictionary")]
  InvalidUserDictionaryErr,
  #[error("not found grammar")]
  NotFoundGrammarErr,
  #[error("{0}")]
  IOError(#[from] IOError),
  #[error("{0}")]
  DictionaryHeaderErr(#[from] DictionaryHeaderErr),
  #[error("{0}")]
  LexiconErr(#[from] LexiconErr),
}

pub struct BinaryDictionary {
  pub grammar: Grammar,
  header: DictionaryHeader,
  pub lexicon: DoubleArrayLexicon,
}

impl BinaryDictionary {
  fn new(
    grammar: Grammar,
    header: DictionaryHeader,
    lexicon: DoubleArrayLexicon,
  ) -> BinaryDictionary {
    BinaryDictionary {
      grammar,
      header,
      lexicon,
    }
  }
  pub fn read_dictionary_from_reader<R: Seek + BufRead>(
    reader: &mut R,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    let header = DictionaryHeader::from_reader(reader)?;

    if SYSTEM_DICT_VERSION_1 != header.version
      && SYSTEM_DICT_VERSION_2 != header.version
      && USER_DICT_VERSION_1 != header.version
      && USER_DICT_VERSION_2 != header.version
      && USER_DICT_VERSION_3 != header.version
    {
      return Err(ReadDictionaryErr::InvalidDictionaryVersionErr);
    }
    if header.version == USER_DICT_VERSION_1 {
      return Err(ReadDictionaryErr::NotFoundGrammarErr);
    }
    let grammar = Grammar::from_reader(reader)?;

    let lexicon = DoubleArrayLexicon::from_reader(reader)?;
    Ok(BinaryDictionary::new(grammar, header, lexicon))
  }
  pub fn from_system_dictionary<P: AsRef<Path>>(
    filename: P,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    let mut reader = BufReader::new(File::open(filename)?);
    let dictionary = BinaryDictionary::read_dictionary_from_reader(&mut reader)?;
    if dictionary.header.version != SYSTEM_DICT_VERSION_1 && dictionary.header.version != SYSTEM_DICT_VERSION_2 {
      return Err(ReadDictionaryErr::InvalidSystemDictionaryErr);
    }
    Ok(dictionary)
  }
  pub fn from_user_dictionary<P: AsRef<Path>>(
    filename: P,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    let mut reader = BufReader::new(File::open(filename)?);
    let dictionary = BinaryDictionary::read_dictionary_from_reader(&mut reader)?;
    if USER_DICT_VERSION_1 != dictionary.header.version
      && USER_DICT_VERSION_2 != dictionary.header.version
    {
      return Err(ReadDictionaryErr::InvalidUserDictionaryErr);
    }
    Ok(dictionary)
  }
}
