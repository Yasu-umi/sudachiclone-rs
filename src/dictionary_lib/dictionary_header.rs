use std::io::{BufRead, Cursor, Error as IOError, Seek, SeekFrom, Write};
use std::string::FromUtf8Error;
use std::time::{SystemTime, UNIX_EPOCH};

use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

use super::io::LittleEndianWrite;

pub struct DictionaryHeader {
  pub version: u64,
  pub create_time: u64,
  pub description: String,
}

const DESCRIPTION_SIZE: usize = 256;
const STORAGE_SIZE: usize = 8 + 8 + DESCRIPTION_SIZE;

#[derive(Error, Debug)]
pub enum DictionaryHeaderErr {
  #[error("description is too long")]
  DescriptionTooLongErr,
  #[error("{0}")]
  IOError(#[from] IOError),
  #[error("{0}")]
  FromUtf8Error(#[from] FromUtf8Error),
}

impl DictionaryHeader {
  pub fn new(version: u64, create_time: u64, description: String) -> DictionaryHeader {
    DictionaryHeader {
      version,
      create_time,
      description,
    }
  }
  pub fn from_reader<R: BufRead + Seek>(
    reader: &mut R,
  ) -> Result<DictionaryHeader, DictionaryHeaderErr> {
    let offset = reader.seek(SeekFrom::Current(0))?;
    let version = reader.read_u64::<LittleEndian>()?;
    let create_time = reader.read_u64::<LittleEndian>()?;
    let mut buf = Vec::with_capacity(DESCRIPTION_SIZE);
    reader.read_until(0u8, &mut buf)?;
    buf.pop().unwrap();
    if buf.len() > DESCRIPTION_SIZE {
      buf.truncate(DESCRIPTION_SIZE);
    }
    reader.seek(SeekFrom::Start(offset + STORAGE_SIZE as u64))?;
    // buf.pop().unwrap();
    let description = String::from_utf8(buf)?;

    Ok(DictionaryHeader {
      version,
      create_time,
      description,
    })
  }
  pub fn get_time() -> u64 {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_secs()
  }
  pub fn to_bytes(&self) -> Result<Vec<u8>, DictionaryHeaderErr> {
    let mut cursor = Cursor::new(vec![0; 16 + DESCRIPTION_SIZE]);
    cursor.write_u64(self.version)?;
    cursor.write_u64(self.create_time)?;
    let bdesc = self.description.as_bytes();
    if bdesc.len() > DESCRIPTION_SIZE {
      return Err(DictionaryHeaderErr::DescriptionTooLongErr);
    }
    cursor.write_all(bdesc)?;
    Ok(cursor.into_inner())
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use super::*;
  use crate::dictionary_lib::system_dictionary_version::SYSTEM_DICT_VERSION;
  use std::fs::File;
  use std::io::BufReader;
  use std::path::PathBuf;
  use std::str::FromStr;

  pub fn read_header() -> DictionaryHeader {
    DictionaryHeader::from_reader(&mut BufReader::new(
      File::open(
        PathBuf::from_str(file!())
          .unwrap()
          .parent()
          .unwrap()
          .parent()
          .unwrap()
          .join("resources/test/system.dic")
          .as_path(),
      )
      .unwrap(),
    ))
    .unwrap()
  }

  #[test]
  fn test_version() {
    let header = read_header();
    assert_eq!(header.version, SYSTEM_DICT_VERSION);
  }

  #[test]
  fn test_create_time() {
    let header = read_header();
    assert!(header.create_time > 0);
  }

  #[test]
  fn test_description() {
    let header = read_header();
    assert_eq!(
      header.description,
      "the system dictionary for the unit tests",
    );
  }
}
