use std::io::{BufRead, Error as IoError};

use byteorder::{LittleEndian, ReadBytesExt};

pub struct WordIdTable {
  bytes: Vec<u8>,
}

impl WordIdTable {
  pub fn from_reader<R: BufRead>(reader: &mut R) -> Result<WordIdTable, IoError> {
    let size = reader.read_u32::<LittleEndian>()? as usize;
    let mut bytes = vec![0u8; size];
    reader.read_exact(&mut bytes)?;
    Ok(WordIdTable { bytes })
  }
  pub fn get(&self, index: usize) -> Vec<usize> {
    let len = self.bytes[index] as usize;
    let offset = index + 1;
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
      result.push(u32::from_le_bytes([
        self.bytes[(offset + i * 4)],
        self.bytes[(offset + i * 4 + 1)],
        self.bytes[(offset + i * 4 + 2)],
        self.bytes[(offset + i * 4 + 3)],
      ]) as usize);
    }
    result
  }
}
