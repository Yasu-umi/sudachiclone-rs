use super::double_array_lexicon::DoubleArrayLexicon;
use super::lexicon::Lexicon;
use super::word_info::WordInfo;

const MAX_DICTIONARIES: usize = 16;

pub struct LexiconSet {
  lexicons: Vec<DoubleArrayLexicon>,
  pos_offsets: Vec<usize>,
}

impl LexiconSet {
  pub fn new(system_lexicon: DoubleArrayLexicon) -> LexiconSet {
    LexiconSet {
      lexicons: vec![system_lexicon],
      pos_offsets: vec![0],
    }
  }
  pub fn is_full(&self) -> bool {
    self.lexicons.len() >= MAX_DICTIONARIES
  }
  fn build_word_id(&self, dict_id: usize, word_id: usize) -> usize {
    if word_id > 0x0FFF_FFFF {
      panic!("word id is too large: {}", word_id)
    }
    if dict_id > self.lexicons.len() {
      panic!("dict id is too large: {}", word_id)
    }
    (dict_id << 28) | word_id
  }
  pub fn first(&self) -> &DoubleArrayLexicon {
    &self.lexicons[0]
  }
  pub fn add(&mut self, lexicon: DoubleArrayLexicon, pos_offset: usize) {
    if !self.lexicons.contains(&lexicon) {
      self.lexicons.push(lexicon);
      self.pos_offsets.push(pos_offset);
    }
  }
  fn _lookup(&self, text: &[u8], offset: usize) -> Vec<(usize, usize)> {
    let mut res = vec![];
    let mut indices: Vec<usize> = (1..self.lexicons.len()).collect();
    indices.push(0);
    for dict_id in indices {
      let pairs = self.lexicons[dict_id].lookup(text, offset);
      for (word_id, length) in pairs {
        res.push((self.build_word_id(dict_id, word_id), length));
      }
    }
    res
  }
  fn convert_split(&self, split: Vec<i32>, dict_id: usize) -> Vec<i32> {
    split
      .into_iter()
      .map(|v| {
        if v > 0 && self.get_dictionary_id(v as usize) > 0 {
          self.build_word_id(dict_id, get_word_id(v as usize)) as i32
        } else {
          v
        }
      })
      .collect()
  }
  pub fn lookup(&self, text: &[u8], offset: usize) -> Vec<(usize, usize)> {
    if self.lexicons.len() == 1 {
      return self.lexicons[0].lookup(text, offset);
    }
    self._lookup(text, offset)
  }
  pub fn get_left_id(&self, word_id: usize) -> i16 {
    self.lexicons[self.get_dictionary_id(word_id) as usize].get_left_id(get_word_id(word_id))
  }
  pub fn get_right_id(&self, word_id: usize) -> i16 {
    self.lexicons[self.get_dictionary_id(word_id) as usize].get_right_id(get_word_id(word_id))
  }
  pub fn get_cost(&self, word_id: usize) -> i16 {
    self.lexicons[self.get_dictionary_id(word_id) as usize].get_cost(get_word_id(word_id))
  }
  pub fn get_word_info(&self, word_id: usize) -> WordInfo {
    let dict_id = self.get_dictionary_id(word_id);
    let mut word_info = self.lexicons[dict_id].get_word_info(get_word_id(word_id));
    let pos_id = word_info.pos_id;
    // user defined part-of-speech
    if dict_id > 0 && pos_id >= self.pos_offsets[1] as i16 {
      word_info.pos_id =
        word_info.pos_id - (self.pos_offsets[1] as i16) + (self.pos_offsets[dict_id] as i16);
    }
    word_info.a_unit_split = self.convert_split(word_info.a_unit_split, dict_id);
    word_info.b_unit_split = self.convert_split(word_info.b_unit_split, dict_id);
    word_info.word_structure = self.convert_split(word_info.word_structure, dict_id);
    word_info
  }
  pub fn get_dictionary_id(&self, word_id: usize) -> usize {
    word_id >> 28
  }
}

fn get_word_id(word_id: usize) -> usize {
  0x0FFF_FFFF & word_id
}
