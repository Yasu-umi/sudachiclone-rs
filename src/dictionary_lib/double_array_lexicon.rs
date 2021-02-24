use std::cmp::{max, min};
use std::io::{BufRead, Seek};

use byteorder::{LittleEndian, ReadBytesExt};
use rand::Rng;

use super::lexicon::{GetWordId, Lexicon, LexiconErr, Size};
use super::word_id_table::WordIdTable;
use super::word_info::WordInfo;
use super::word_info_list::WordInfoList;
use super::word_parameter_list::WordParameterList;
use crate::darts::DoubleArrayTrie;
use crate::tokenizer::CanTokenize;

pub const SIGNED_SHORT_MIN: i16 = std::i16::MIN;
pub const SIGNED_SHORT_MAX: i16 = std::i16::MAX;
pub const USER_DICT_COST_PER_MORPH: i16 = -20;

pub struct DoubleArrayLexicon {
  id: u64,
  trie: DoubleArrayTrie,
  word_id_table: WordIdTable,
  word_params: WordParameterList,
  word_infos: WordInfoList,
}

impl<L: AsRef<DoubleArrayLexicon>> Lexicon for L {
  fn lookup(&self, text: &[u8], offset: usize) -> Vec<(usize, usize)> {
    let key = &text[offset..];
    let result = self.as_ref().trie.common_prefix_search(key);
    result
      .into_iter()
      .map(|(index, mut length)| {
        let word_ids = self.as_ref().word_id_table.get(index as usize);
        length += offset;
        word_ids
          .into_iter()
          .map(|word_id| (word_id, length))
          .collect::<Vec<(usize, usize)>>()
      })
      .flatten()
      .collect()
  }
  fn get_left_id(&self, word_id: usize) -> i16 {
    self.as_ref().word_params.get_left_id(word_id)
  }
  fn get_right_id(&self, word_id: usize) -> i16 {
    self.as_ref().word_params.get_right_id(word_id)
  }
  fn get_cost(&self, word_id: usize) -> i16 {
    self.as_ref().word_params.get_cost(word_id)
  }
  fn get_word_info(&self, word_id: usize) -> WordInfo {
    self.as_ref().word_infos.get_word_info(word_id)
  }
  fn get_dictionary_id(&self, _word_id: usize) -> usize {
    0
  }
}

impl Size for DoubleArrayLexicon {
  fn size(&self) -> usize {
    self.word_params.get_size()
  }
}

impl GetWordId for DoubleArrayLexicon {
  fn get_word_id(
    &self,
    headword: &str,
    pos_id: u16,
    reading_form: &str,
  ) -> Result<usize, LexiconErr> {
    for word_id in 0..self.word_infos.size() {
      let info = self.word_infos.get_word_info(word_id);
      if info.surface == headword
        && info.pos_id == pos_id as i16
        && info.reading_form == reading_form
      {
        return Ok(word_id);
      }
    }
    Err(LexiconErr::NotFoundWordIdErr)
  }
}

impl DoubleArrayLexicon {
  pub fn from_reader<R: BufRead + Seek>(reader: &mut R) -> Result<DoubleArrayLexicon, LexiconErr> {
    let size = reader.read_u32::<LittleEndian>()? as usize;

    let mut trie = DoubleArrayTrie::default();
    let mut buf = vec![0u8; size * 4];
    reader.read_exact(&mut buf)?;
    trie.set_array(&buf, size);

    let word_id_table = WordIdTable::from_reader(reader)?;

    let word_params = WordParameterList::from_reader(reader)?;

    let word_infos = WordInfoList::from_reader(reader, word_params.get_size())?;

    Ok(DoubleArrayLexicon {
      id: rand::thread_rng().gen(),
      trie,
      word_id_table,
      word_params,
      word_infos,
    })
  }
  pub fn calculate_cost<T: CanTokenize>(&mut self, tokenizer: T) {
    for word_id in 0..self.word_params.get_size() {
      if self.get_cost(word_id) != SIGNED_SHORT_MIN {
        continue;
      }
      let surface = self.get_word_info(word_id).surface;
      let ms = tokenizer.tokenize(&surface, None, None);
      if let Some(ms) = ms {
        let mut cost = ms.get_internal_cost() + USER_DICT_COST_PER_MORPH * ms.len() as i16;
        cost = min(cost, SIGNED_SHORT_MAX);
        cost = max(cost, SIGNED_SHORT_MIN);
        self.word_params.set_cost(word_id, cost);
      }
    }
  }
}

impl PartialEq for DoubleArrayLexicon {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl AsRef<DoubleArrayLexicon> for DoubleArrayLexicon {
  fn as_ref(&self) -> &DoubleArrayLexicon {
    self
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use super::*;
  use crate::dictionary_lib::dictionary_header::DictionaryHeader;
  use std::fs::File;
  use std::io::{BufReader, Seek, SeekFrom};
  use std::path::PathBuf;
  use std::str::FromStr;

  fn read_lexicon() -> DoubleArrayLexicon {
    let mut reader = BufReader::new(
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
    );
    DictionaryHeader::from_reader(&mut reader).unwrap();
    reader.seek(SeekFrom::Current(470)).unwrap();
    DoubleArrayLexicon::from_reader(&mut reader).unwrap()
  }

  #[test]
  fn test_lookup() {
    let lexicon = read_lexicon();
    let results1 = lexicon.lookup("東京都".as_bytes(), 0);
    assert_eq!((4, 3), results1[0]); // 東
    assert_eq!((5, 6), results1[1]); // 東京
    assert_eq!((6, 9), results1[2]); // 東京都
    assert_eq!(3, results1.len());

    let results2 = lexicon.lookup("東京都に".as_bytes(), 9);
    assert_eq!((1, 12), results2[0]); // に(接続助詞)
    assert_eq!((2, 12), results2[1]); // に(格助詞)
    assert_eq!(2, results2.len());

    let results3 = lexicon.lookup("あれ".as_bytes(), 0);
    assert_eq!(0, results3.len());
  }

  #[test]
  fn test_parameters() {
    // た
    let mut lexicon = read_lexicon();
    assert_eq!(1, lexicon.get_left_id(0));
    assert_eq!(1, lexicon.get_right_id(0));
    assert_eq!(8729, lexicon.get_cost(0));

    // 東京都
    lexicon = read_lexicon();
    assert_eq!(6, lexicon.get_left_id(6));
    assert_eq!(8, lexicon.get_right_id(6));
    assert_eq!(5320, lexicon.get_cost(6));

    // 都
    lexicon = read_lexicon();
    assert_eq!(8, lexicon.get_left_id(9));
    assert_eq!(8, lexicon.get_right_id(9));
    assert_eq!(2914, lexicon.get_cost(9));
  }

  #[test]
  fn test_word_info() {
    let lexicon = read_lexicon();

    let word_info = lexicon.get_word_info(0);
    assert_eq!("た", &word_info.surface);
    assert_eq!(3, word_info.head_word_length);
    assert_eq!(0, word_info.pos_id);
    assert_eq!("た", &word_info.normalized_form);
    assert_eq!(-1, word_info.dictionary_form_word_id);
    assert_eq!("た", &word_info.dictionary_form);
    assert_eq!("タ", &word_info.reading_form);
    assert_eq!(vec![0i32; 0], word_info.a_unit_split);
    assert_eq!(vec![0i32; 0], word_info.b_unit_split);
    assert_eq!(vec![0i32; 0], word_info.word_structure);

    let word_info = lexicon.get_word_info(8);
    assert_eq!("行っ", &word_info.surface);
    assert_eq!("行く", &word_info.normalized_form);
    assert_eq!(7, word_info.dictionary_form_word_id);
    assert_eq!("行く", &word_info.dictionary_form);

    let word_info = lexicon.get_word_info(6);
    assert_eq!("東京都", word_info.surface);
    assert_eq!(vec![5, 9], word_info.a_unit_split);
    assert_eq!(vec![0i32; 0], word_info.b_unit_split);
    assert_eq!(vec![5, 9], word_info.word_structure);
  }

  #[test]
  fn test_wordinfo_with_longword() {
    let lexicon = read_lexicon();
    let word_info = lexicon.get_word_info(36);
    assert_eq!(300, word_info.surface.chars().count());
    assert_eq!(300, word_info.head_word_length);
    assert_eq!(300, word_info.normalized_form.chars().count());
    assert_eq!(-1, word_info.dictionary_form_word_id);
    assert_eq!(300, word_info.dictionary_form.chars().count());
    assert_eq!(570, word_info.reading_form.chars().count());
  }

  #[test]
  fn test_size() {
    let lexicon = read_lexicon();
    assert_eq!(38, lexicon.size());
  }
}
