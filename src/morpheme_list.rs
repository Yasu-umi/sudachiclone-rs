use std::sync::{Arc, Mutex};

use super::dictionary_lib::grammar::Grammar;
use super::dictionary_lib::word_info::WordInfo;
use super::lattice_node::LatticeNode;
use super::morpheme::Morpheme;
use super::utf8_input_text::UTF8InputText;

pub struct MorphemeList {
  input_text: Arc<Mutex<UTF8InputText>>,
  grammar: Arc<Mutex<Grammar>>,
  path: Vec<Arc<Mutex<LatticeNode>>>,
}

impl MorphemeList {
  pub fn new(
    input_text: UTF8InputText,
    grammar: Arc<Mutex<Grammar>>,
    path: Vec<Arc<Mutex<LatticeNode>>>,
  ) -> MorphemeList {
    MorphemeList {
      input_text: Arc::new(Mutex::new(input_text)),
      grammar,
      path,
    }
  }
  pub fn get_start(&self, index: usize) -> usize {
    self
      .input_text
      .lock()
      .unwrap()
      .get_original_index(self.path[index].lock().unwrap().get_start())
  }
  pub fn get_end(&self, index: usize) -> usize {
    self
      .input_text
      .lock()
      .unwrap()
      .get_original_index(self.path[index].lock().unwrap().get_end())
  }
  pub fn get_surface(&self, index: usize) -> String {
    let start = self.get_start(index);
    let end = self.get_end(index);
    self.input_text.lock().unwrap().get_original_text()[start..end].to_string()
  }
  pub fn get_internal_cost(&self) -> i16 {
    (self.path.last().unwrap().lock().unwrap().get_path_cost()
      - self.path[0].lock().unwrap().get_path_cost()) as i16
  }
  pub fn len(&self) -> usize {
    self.path.len()
  }
  pub fn is_empty(&self) -> bool {
    self.path.is_empty()
  }
  pub fn iter(&self) -> MorphemeIterator {
    MorphemeIterator {
      list: self,
      index: 0,
    }
  }
  pub fn get_word_info(&self, index: usize) -> WordInfo {
    self.path[index].lock().unwrap().get_word_info()
  }
  pub fn get(&self, index: usize) -> Option<Morpheme> {
    let node = self.path.get(index);
    node.map(|node| {
      let word_info = self.get_word_info(index);
      Morpheme::new(
        Arc::clone(&self.input_text),
        word_info,
        Arc::clone(&self.grammar),
        Arc::clone(node),
      )
    })
  }
}

pub struct MorphemeIterator<'a> {
  list: &'a MorphemeList,
  index: usize,
}

impl<'a> Iterator for MorphemeIterator<'a> {
  type Item = Morpheme;
  fn next(&mut self) -> Option<Self::Item> {
    let index = self.index;
    self.index += 1;
    self.list.get(index)
  }
}

impl IntoIterator for MorphemeList {
  type Item = Morpheme;
  type IntoIter = MorphemeIntoIterator;

  fn into_iter(self) -> Self::IntoIter {
    MorphemeIntoIterator {
      list: self,
      index: 0,
    }
  }
}

pub struct MorphemeIntoIterator {
  list: MorphemeList,
  index: usize,
}

impl Iterator for MorphemeIntoIterator {
  type Item = Morpheme;
  fn next(&mut self) -> Option<Self::Item> {
    let index = self.index;
    self.index += 1;
    self.list.get(index)
  }
}
