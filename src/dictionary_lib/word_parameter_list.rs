use std::io::{BufRead, Error as IOError};

use byteorder::{LittleEndian, ReadBytesExt};

pub struct WordParameterList {
  size: usize,
  array_view: Vec<i16>,
}

// const ELEMENT_SIZE: usize = 2 * 3;
const ELEMENT_SIZE_AS_SHORT: usize = 3;

impl WordParameterList {
  pub fn from_reader<R: BufRead>(reader: &mut R) -> Result<WordParameterList, IOError> {
    let size = reader.read_u32::<LittleEndian>()? as usize;
    let mut array_view = vec![0i16; ELEMENT_SIZE_AS_SHORT * size];
    reader.read_i16_into::<LittleEndian>(&mut array_view)?;
    Ok(WordParameterList { size, array_view })
  }
  pub fn get_size(&self) -> usize {
    self.size
  }
  pub fn get_left_id(&self, word_id: usize) -> i16 {
    self.array_view[ELEMENT_SIZE_AS_SHORT * word_id]
  }
  pub fn get_right_id(&self, word_id: usize) -> i16 {
    self.array_view[ELEMENT_SIZE_AS_SHORT * word_id + 1]
  }
  pub fn get_cost(&self, word_id: usize) -> i16 {
    self.array_view[ELEMENT_SIZE_AS_SHORT * word_id + 2]
  }
  pub fn set_cost(&mut self, word_id: usize, cost: i16) {
    self.array_view[ELEMENT_SIZE_AS_SHORT * word_id + 2] = cost;
  }
}
