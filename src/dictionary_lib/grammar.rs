use std::io::{BufRead, Error as IOError, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use encoding_rs::UTF_16LE;

use super::character_category::CharacterCategory;

pub const INHIBITED_CONNECTION: i16 = 0x7fff;

pub const POS_DEPTH: usize = 6;

pub struct Grammar {
  bos_parameter: [u32; 3],
  eos_parameter: [u32; 3],
  character_category: Option<CharacterCategory>,
  pos_list: Vec<Vec<String>>,
  storage_size: usize,
  matrix_view: Vec<Vec<i16>>,
}

impl Grammar {
  pub fn from_reader<R: Seek + BufRead>(reader: &mut R) -> Result<Grammar, IOError> {
    let offset = reader.seek(SeekFrom::Current(0))? as usize;
    let pos_size = reader.read_i16::<LittleEndian>()? as usize;
    let mut pos_list = vec![Vec::with_capacity(6); pos_size];
    for pos in pos_list.iter_mut() {
      for _ in 0..POS_DEPTH {
        let size = reader.read_u8()? as usize;
        let mut buf = vec![0u8; size * 2];
        reader.read_exact(&mut buf)?;
        let (p, _, _) = UTF_16LE.decode(&buf);
        pos.push(p.to_string());
      }
    }
    let left_id_size = reader.read_i16::<LittleEndian>()? as usize;
    let right_id_size = reader.read_i16::<LittleEndian>()? as usize;
    let connect_table_offset = reader.seek(SeekFrom::Current(0))? as usize;

    let storage_size = (connect_table_offset - offset) + 2 * left_id_size * right_id_size;

    let matrix_view = if left_id_size == 0 || right_id_size == 0 {
      vec![]
    } else {
      let mut buf = vec![0i16; left_id_size * right_id_size];
      reader.read_i16_into::<LittleEndian>(&mut buf)?;
      let mut matrix_view = vec![vec![0; right_id_size]; left_id_size];
      for i in 0..left_id_size {
        for j in 0..right_id_size {
          matrix_view[i][j] = buf[(i * left_id_size) + j];
        }
      }
      matrix_view
    };

    Ok(Grammar {
      bos_parameter: [0, 0, 0],
      eos_parameter: [0, 0, 0],
      character_category: None,
      pos_list,
      storage_size,
      matrix_view,
    })
  }
  pub fn get_storage_size(&self) -> usize {
    self.storage_size
  }
  pub fn get_part_of_speech_size(&self) -> usize {
    self.pos_list.len()
  }
  pub fn get_part_of_speech_string(&self, pos_id: usize) -> &Vec<String> {
    &self.pos_list[pos_id]
  }
  pub fn get_connect_cost(&self, left: usize, right: usize) -> i16 {
    self.matrix_view[right][left]
  }
  pub fn get_bos_parameter(&self) -> [u32; 3] {
    self.bos_parameter
  }
  pub fn get_eos_parameter(&self) -> [u32; 3] {
    self.eos_parameter
  }
  pub fn add_pos_list(&mut self, grammar: &Grammar) {
    self.pos_list.extend_from_slice(&grammar.pos_list);
  }
}

pub trait GetPartOfSpeech {
  fn get_part_of_speech_id(&self, pos: &[&str]) -> Option<usize>;
  fn get_part_of_speech_size(&self) -> usize;
}
impl GetPartOfSpeech for Grammar {
  fn get_part_of_speech_size(&self) -> usize {
    self.pos_list.len()
  }
  fn get_part_of_speech_id(&self, pos: &[&str]) -> Option<usize> {
    self.pos_list.iter().position(|p| p.iter().eq(pos))
  }
}

pub trait GetCharacterCategory {
  fn get_character_category(&self) -> &Option<CharacterCategory>;
}
impl GetCharacterCategory for Grammar {
  fn get_character_category(&self) -> &Option<CharacterCategory> {
    &self.character_category
  }
}
impl GetCharacterCategory for &Grammar {
  fn get_character_category(&self) -> &Option<CharacterCategory> {
    &self.character_category
  }
}

pub trait SetCharacterCategory {
  fn set_character_category(&mut self, character_category: Option<CharacterCategory>);
}
impl SetCharacterCategory for Grammar {
  fn set_character_category(&mut self, character_category: Option<CharacterCategory>) {
    self.character_category = character_category;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use byteorder::{ByteOrder, LittleEndian};
  use std::io::Cursor;

  fn build_grammar() -> Grammar {
    let mut bytes = vec![];
    build_partofspeech(&mut bytes);
    build_connect_table(&mut bytes);

    Grammar::from_reader(&mut Cursor::new(bytes)).unwrap()
  }
  fn build_partofspeech(bytes: &mut Vec<u8>) {
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 3);
    bytes.extend(buf);

    bytes.extend(&[
      7, 66, 0, 79, 0, 83, 0, 47, 0, 69, 0, 79, 0, 83, 0, 1, 42, 0, 1, 42, 0, 1, 42, 0, 1, 42, 0,
      1, 42, 0,
    ]);

    bytes.extend(&[2]);
    bytes.extend(encode_utf16le_bytes("名前"));
    bytes.extend(&[2]);
    bytes.extend(encode_utf16le_bytes("一般"));
    bytes.extend(&[1, 42, 0, 1, 42, 0, 1, 42, 0, 1, 42, 0]);

    bytes.extend(&[2]);
    bytes.extend(encode_utf16le_bytes("動詞"));
    bytes.extend(&[2]);
    bytes.extend(encode_utf16le_bytes("一般"));
    bytes.extend(&[1, 42, 0, 1, 42, 0, 5]);
    bytes.extend(encode_utf16le_bytes("五段-サ行"));
    bytes.extend(&[6]);
    bytes.extend(encode_utf16le_bytes("終止形-一般"));
  }
  fn encode_utf16le_bytes(text: &str) -> Vec<u8> {
    let src: Vec<u16> = text.encode_utf16().collect();
    let mut dst = vec![0; src.len() * 2];
    LittleEndian::write_u16_into(&src, &mut dst);
    dst
  }
  fn build_connect_table(bytes: &mut Vec<u8>) {
    // number of leftId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 3);
    bytes.extend(buf);
    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 3);
    bytes.extend(buf);

    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 0);
    bytes.extend(buf);
    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, -300);
    bytes.extend(buf);
    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 3000);
    bytes.extend(buf);

    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 300);
    bytes.extend(buf);
    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, -500);
    bytes.extend(buf);
    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, -100);
    bytes.extend(buf);

    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, -3000);
    bytes.extend(buf);
    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 200);
    bytes.extend(buf);
    // number of rightId
    let mut buf = vec![0; 2];
    LittleEndian::write_i16(&mut buf, 2000);
    bytes.extend(buf);
  }
  #[test]
  fn test_storage_size() {
    let grammar = build_grammar();
    assert_eq!(grammar.get_storage_size(), 116);
  }
  #[test]
  fn test_get_partofspeech_string() {
    let grammar = build_grammar();
    assert_eq!(6, grammar.get_part_of_speech_string(0).len());
    assert_eq!("BOS/EOS", grammar.get_part_of_speech_string(0)[0]);
    assert_eq!("*", grammar.get_part_of_speech_string(0)[5]);
    assert_eq!("一般", grammar.get_part_of_speech_string(1)[1]);
    assert_eq!("*", grammar.get_part_of_speech_string(1)[5]);
    assert_eq!("五段-サ行", grammar.get_part_of_speech_string(2)[4]);
    assert_eq!("終止形-一般", grammar.get_part_of_speech_string(2)[5]);
  }
  #[test]
  fn test_get_connect_cost() {
    let grammar = build_grammar();
    assert_eq!(0, grammar.get_connect_cost(0, 0));
    assert_eq!(-100, grammar.get_connect_cost(2, 1));
    assert_eq!(200, grammar.get_connect_cost(1, 2));
  }
  #[test]
  fn test_get_bos_parameters() {
    let grammar = build_grammar();
    assert_eq!(0, grammar.get_bos_parameter()[0]);
    assert_eq!(0, grammar.get_bos_parameter()[1]);
    assert_eq!(0, grammar.get_bos_parameter()[2]);
  }
  #[test]
  fn test_get_eos_parameters() {
    let grammar = build_grammar();
    assert_eq!(0, grammar.get_eos_parameter()[0]);
    assert_eq!(0, grammar.get_eos_parameter()[1]);
    assert_eq!(0, grammar.get_eos_parameter()[2]);
  }
}
