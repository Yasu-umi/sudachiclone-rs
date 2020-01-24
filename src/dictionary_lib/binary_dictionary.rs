use std::fs::File;
use std::io::{BufReader, Error as IOError};
use std::path::Path;

use thiserror::Error;

use super::dictionary_header::{DictionaryHeader, DictionaryHeaderErr};
use super::double_array_lexicon::DoubleArrayLexicon;
use super::grammar::Grammar;
use super::lexicon::LexiconErr;
use super::system_dictionary_version::{
  SYSTEM_DICT_VERSION, USER_DICT_VERSION_1, USER_DICT_VERSION_2,
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
  #[error("{self:?}")]
  IOError(#[from] IOError),
  #[error("{self:?}")]
  DictionaryHeaderErr(#[from] DictionaryHeaderErr),
  #[error("{self:?}")]
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
  pub fn read_dictionary<P: AsRef<Path>>(
    filename: P,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    let mut reader = BufReader::new(File::open(filename)?);

    let header = DictionaryHeader::from_reader(&mut reader)?;

    if ![
      SYSTEM_DICT_VERSION,
      USER_DICT_VERSION_1,
      USER_DICT_VERSION_2,
    ]
    .contains(&header.version)
    {
      return Err(ReadDictionaryErr::InvalidDictionaryVersionErr);
    }
    if header.version == USER_DICT_VERSION_1 {
      return Err(ReadDictionaryErr::NotFoundGrammarErr);
    }
    let grammar = Grammar::from_reader(&mut reader)?;

    let lexicon = DoubleArrayLexicon::from_reader(&mut reader)?;
    Ok(BinaryDictionary::new(grammar, header, lexicon))
  }
  pub fn from_system_dictionary<P: AsRef<Path>>(
    filename: P,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    let dictionary = BinaryDictionary::read_dictionary(filename)?;
    if dictionary.header.version != SYSTEM_DICT_VERSION {
      return Err(ReadDictionaryErr::InvalidSystemDictionaryErr);
    }
    Ok(dictionary)
  }
  pub fn from_user_dictionary<P: AsRef<Path>>(
    filename: P,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    let dictionary = BinaryDictionary::read_dictionary(filename)?;
    if ![USER_DICT_VERSION_1, USER_DICT_VERSION_2].contains(&dictionary.header.version) {
      return Err(ReadDictionaryErr::InvalidUserDictionaryErr);
    }
    Ok(dictionary)
  }
}
