use std::io::Cursor;
use std::io::{BufRead, Error as IoError, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use encoding_rs::UTF_16LE;

use super::word_info::WordInfo;

pub struct WordInfoList {
  bytes: Vec<u8>,
  word_size: usize,
  offset: usize,
}

impl WordInfoList {
  pub fn from_reader<R: BufRead + Seek>(
    reader: &mut R,
    word_size: usize,
  ) -> Result<WordInfoList, IoError> {
    let offset = reader.seek(SeekFrom::Current(0))? as usize;
    let mut bytes = vec![];
    reader.read_to_end(&mut bytes)?;
    Ok(WordInfoList {
      bytes,
      word_size,
      offset,
    })
  }
  pub fn get_word_info(&self, word_id: usize) -> WordInfo {
    let offset = self.word_id_to_offset(word_id) as usize;
    let offset = offset - self.offset;
    let (surface, offset) = WordInfoList::buffer_to_string(&self.bytes, offset);

    let (head_word_length, mut offset) = WordInfoList::buffer_to_string_length(&self.bytes, offset);

    let pos_id = Cursor::new(&self.bytes[offset..(offset + 2)])
      .read_u16::<LittleEndian>()
      .unwrap() as i16;
    offset += 2;

    let (mut normalized_form, mut offset) = WordInfoList::buffer_to_string(&self.bytes, offset);
    if normalized_form.is_empty() {
      normalized_form = surface.clone();
    }

    let dictionary_form_word_id = Cursor::new(&self.bytes[offset..(offset + 4)])
      .read_i32::<LittleEndian>()
      .unwrap();
    offset += 4;

    let (mut reading_form, offset) = WordInfoList::buffer_to_string(&self.bytes, offset);
    if reading_form.is_empty() {
      reading_form = surface.clone();
    }

    let (a_unit_split, offset) = WordInfoList::buffer_to_int_array(&self.bytes, offset);
    let (b_unit_split, offset) = WordInfoList::buffer_to_int_array(&self.bytes, offset);
    let (word_structure, _offset) = WordInfoList::buffer_to_int_array(&self.bytes, offset);

    let dictionary_form =
      if dictionary_form_word_id >= 0 && dictionary_form_word_id != word_id as i32 {
        let wi = self.get_word_info(dictionary_form_word_id as usize);
        wi.surface
      } else {
        surface.clone()
      };

    WordInfo {
      surface,
      head_word_length,
      pos_id,
      normalized_form,
      dictionary_form_word_id,
      dictionary_form,
      reading_form,
      a_unit_split,
      b_unit_split,
      word_structure,
    }
  }
  fn word_id_to_offset(&self, word_id: usize) -> u32 {
    let i = 4 * word_id;
    let mut cursor = Cursor::new(&self.bytes[i..i + 4]);
    cursor.read_u32::<LittleEndian>().unwrap()
  }
  fn buffer_to_string_length(bytes: &[u8], offset: usize) -> (usize, usize) {
    let len = bytes[offset] as usize;
    if len < 128 {
      return (len, offset + 1);
    }
    let low = bytes[offset + 1] as usize;
    (((len & 0x7F) << 8) | low, offset + 2)
  }
  fn buffer_to_string(bytes: &[u8], offset: usize) -> (String, usize) {
    let (len, offset) = WordInfoList::buffer_to_string_length(bytes, offset);
    let new_offset = offset + (len * 2);
    let (text, _, _) = UTF_16LE.decode(&bytes[offset..new_offset]);
    ((*text).to_string(), new_offset)
  }
  fn buffer_to_int_array(bytes: &[u8], offset: usize) -> (Vec<i32>, usize) {
    let len = bytes[offset] as usize;
    let vec = (0..len)
      .map(|i| {
        Cursor::new(&bytes[offset + 1 + i * 4..offset + 5 + i * 4])
          .read_i32::<LittleEndian>()
          .unwrap()
      })
      .collect();
    (vec, offset + 1 + len * 4)
  }
  pub fn size(&self) -> usize {
    self.word_size
  }
}
