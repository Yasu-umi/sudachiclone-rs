use std::ops::Deref;
use std::sync::{Arc, Mutex};

use log::{info, log_enabled, set_boxed_logger, Level, Log};

use super::dictionary_lib::category_type::CategoryType;
use super::dictionary_lib::grammar::Grammar;
use super::dictionary_lib::lexicon_set::LexiconSet;
use super::lattice::Lattice;
use super::lattice_node::LatticeNode;
use super::morpheme_list::MorphemeList;
use super::plugin::input_text_plugin::{InputTextPlugin, RewriteInputText};
use super::plugin::oov_provider_plugin::{get_oov, OovProviderPlugin};
use super::plugin::path_rewrite_plugin::{PathRewritePlugin, RewritePath};
use super::utf8_input_text::{InputText, UTF8InputText};
use super::utf8_input_text_builder::UTF8InputTextBuilder;

pub trait CanTokenize {
  fn tokenize<T: AsRef<str>>(
    &self,
    text: T,
    mode: &Option<SplitMode>,
    logger: Option<Box<dyn Log>>,
  ) -> Option<MorphemeList>;
}

#[derive(PartialEq)]
pub enum SplitMode {
  A,
  B,
  C,
}

pub struct Tokenizer {
  grammar: Arc<Mutex<Grammar>>,
  lexicon_set: Arc<Mutex<LexiconSet>>,
  input_text_plugins: Arc<Vec<InputTextPlugin>>,
  oov_provider_plugins: Arc<Vec<OovProviderPlugin>>,
  path_rewrite_plugins: Arc<Vec<PathRewritePlugin>>,
}

impl Tokenizer {
  pub fn new(
    grammar: Arc<Mutex<Grammar>>,
    lexicon_set: Arc<Mutex<LexiconSet>>,
    input_text_plugins: Arc<Vec<InputTextPlugin>>,
    oov_provider_plugins: Arc<Vec<OovProviderPlugin>>,
    path_rewrite_plugins: Arc<Vec<PathRewritePlugin>>,
  ) -> Tokenizer {
    Tokenizer {
      grammar,
      lexicon_set,
      input_text_plugins,
      oov_provider_plugins,
      path_rewrite_plugins,
    }
  }
  fn build_lattice(&self, input: &UTF8InputText) -> Lattice {
    let mut lattice = Lattice::new(Arc::clone(&self.grammar));
    let bytes = input.get_byte_text();
    let len = bytes.len();
    lattice.resize(len);
    for i in 0..len {
      if !input.can_bow(i) || !lattice.has_previous_node(i) {
        continue;
      }
      let mut has_words = false;
      let lexicon_set = self.lexicon_set.lock().unwrap();
      for (word_id, end) in lexicon_set.lookup(bytes, i) {
        if end < len && !input.can_bow(end) {
          continue;
        }
        has_words = true;
        let node = LatticeNode::new(
          Some(Arc::clone(&self.lexicon_set)),
          lexicon_set.get_left_id(word_id) as u32,
          lexicon_set.get_right_id(word_id) as u32,
          lexicon_set.get_cost(word_id) as i32,
          word_id,
        );
        lattice.insert(i, end, Arc::new(Mutex::new(node)));
      }
      // OOV
      if !input
        .get_char_category_types(i, None)
        .contains(&CategoryType::NOOOVBOW)
      {
        for oov_plugin in self.oov_provider_plugins.iter() {
          process_oov(oov_plugin.deref(), input, i, &mut has_words, &mut lattice);
        }
      }
      if !has_words {
        if let Some(oov_plugin) = self.oov_provider_plugins.last() {
          process_oov(oov_plugin.deref(), input, i, &mut has_words, &mut lattice);
        }
      }
      if !has_words {
        panic!(format!("there is no morpheme at {}", i));
      }
    }
    lattice.connect_eos_node();
    lattice
  }
  fn split_path(
    &self,
    path: Vec<Arc<Mutex<LatticeNode>>>,
    mode: &SplitMode,
  ) -> Vec<Arc<Mutex<LatticeNode>>> {
    if mode == &SplitMode::C {
      return path;
    }
    let mut new_path = vec![];
    for node in path {
      let word_ids = if mode == &SplitMode::A {
        node.lock().unwrap().get_word_info().a_unit_split
      } else {
        node.lock().unwrap().get_word_info().b_unit_split
      };
      if word_ids.len() <= 1 {
        new_path.push(node);
      } else {
        let mut offset = node.lock().unwrap().get_start();
        for word_id in word_ids {
          let mut node = LatticeNode::new(
            Some(Arc::clone(&self.lexicon_set)),
            0,
            0,
            0,
            word_id as usize,
          );
          node.start = offset;
          offset += node.get_word_info().head_word_length;
          node.end = offset;
          new_path.push(Arc::new(Mutex::new(node)));
        }
      }
    }
    new_path
  }
}

impl<'a, C: CanTokenize + ?Sized> CanTokenize for &'a C {
  fn tokenize<T: AsRef<str>>(
    &self,
    text: T,
    mode: &Option<SplitMode>,
    logger: Option<Box<dyn Log>>,
  ) -> Option<MorphemeList> {
    (**self).tokenize(text, mode, logger)
  }
}

impl CanTokenize for Tokenizer {
  fn tokenize<T: AsRef<str>>(
    &self,
    text: T,
    mode: &Option<SplitMode>,
    logger: Option<Box<dyn Log>>,
  ) -> Option<MorphemeList> {
    if text.as_ref().is_empty() {
      return None;
    }
    if let Some(logger) = logger {
      set_boxed_logger(logger).unwrap();
    }

    let mode = mode.as_ref().unwrap_or(&SplitMode::C);
    let mut builder = UTF8InputTextBuilder::new(text.as_ref(), Arc::clone(&self.grammar));
    for plugin in self.input_text_plugins.iter() {
      if plugin.rewrite(&mut builder).is_err() {
        return None;
      }
    }
    let input = builder.build();
    info!("=== Input dump:\n{}", input.get_text());

    let mut lattice = self.build_lattice(&input);
    info!("=== Lattice dump:");
    lattice.log();

    let path = lattice.get_best_path();
    info!("=== Before Rewriting:");
    log_path(&path);

    for plugin in self.path_rewrite_plugins.iter() {
      plugin.rewrite(&input, &path, &lattice);
    }
    lattice.clear();

    let path = self.split_path(path, mode);
    info!("=== After Rewriting:");
    log_path(&path);
    info!("===");

    Some(MorphemeList::new(input, Arc::clone(&self.grammar), path))
  }
}

fn process_oov(
  oov_plugin: &OovProviderPlugin,
  input: &UTF8InputText,
  i: usize,
  has_words: &mut bool,
  lattice: &mut Lattice,
) {
  for node in get_oov(oov_plugin, input, i, *has_words) {
    *has_words = true;
    let (start, end) = {
      let _node = node.lock().unwrap();
      (_node.get_start(), _node.get_end())
    };
    lattice.insert(start, end, node);
  }
}

fn log_path(path: &[Arc<Mutex<LatticeNode>>]) {
  if !log_enabled!(Level::Info) {
    return;
  }
  for (i, node) in path.iter().enumerate() {
    info!("{}: {:?}", i, node.lock().unwrap());
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use super::*;
  use crate::dictionary::Dictionary;
  use std::path::PathBuf;
  use std::str::FromStr;

  fn build_dictionary() -> Dictionary {
    let resource_dir = PathBuf::from_str(file!())
      .unwrap()
      .parent()
      .unwrap()
      .join("resources/test");
    let config_path = resource_dir.join("sudachi.json");
    Dictionary::setup(
      Some(config_path.to_str().unwrap()),
      Some(resource_dir.to_str().unwrap()),
    )
    .unwrap()
  }

  fn build_tokenizer() -> (Dictionary, Tokenizer) {
    let dictionary = build_dictionary();
    let tokenizer = dictionary.create();
    (dictionary, tokenizer)
  }

  #[test]
  fn test_tokenize_small_katanana_only() {
    let (_, tokenizer) = &build_tokenizer();
    let morpheme_list = tokenizer.tokenize("ァ", &None, None).unwrap();
    assert_eq!(1, morpheme_list.len());
  }

  #[test]
  fn test_part_of_speech() {
    let (dictionary, tokenizer) = &build_tokenizer();
    let morpheme_list = tokenizer.tokenize("京都", &None, None).unwrap();
    assert_eq!(1, morpheme_list.len());
    let pid = morpheme_list.get(0).unwrap().part_of_speech_id() as usize;
    assert!(
      dictionary
        .get_grammar()
        .lock()
        .unwrap()
        .get_part_of_speech_size()
        > pid
    );
    assert_eq!(
      &morpheme_list.get(0).unwrap().part_of_speech(),
      dictionary
        .get_grammar()
        .lock()
        .unwrap()
        .get_part_of_speech_string(pid)
    );
  }

  #[test]
  fn test_get_word_id() {
    let (_, tokenizer) = &build_tokenizer();
    let morpheme_list = tokenizer.tokenize("京都", &None, None).unwrap();
    let morpheme = morpheme_list.get(0).unwrap();
    assert_eq!(1, morpheme_list.len());
    assert_eq!(
      vec![
        String::from("名詞"),
        String::from("固有名詞"),
        String::from("地名"),
        String::from("一般"),
        String::from("*"),
        String::from("*")
      ],
      morpheme.part_of_speech()
    );

    let word_id = &morpheme.get_word_id();
    let morpheme_list = tokenizer.tokenize("ぴらる", &None, None).unwrap();
    let morpheme = morpheme_list.get(0).unwrap();
    assert_eq!(1, morpheme_list.len());
    assert!(word_id != &morpheme.get_word_id());
    assert_eq!(
      vec![
        String::from("名詞"),
        String::from("普通名詞"),
        String::from("一般"),
        String::from("*"),
        String::from("*"),
        String::from("*")
      ],
      morpheme.part_of_speech()
    );

    let morpheme_list = tokenizer.tokenize("京", &None, None).unwrap();
    assert_eq!(1, morpheme_list.len());
  }

  #[test]
  fn test_get_dictionary_id() {
    let (_, tokenizer) = &build_tokenizer();
    let morpheme_list = tokenizer.tokenize("京都", &None, None).unwrap();
    let morpheme = morpheme_list.get(0).unwrap();
    assert_eq!(Some(0), morpheme.dictionary_id());

    let morpheme_list = tokenizer.tokenize("ぴらる", &None, None).unwrap();
    let morpheme = morpheme_list.get(0).unwrap();
    assert_eq!(Some(1), morpheme.dictionary_id());

    let morpheme_list = tokenizer.tokenize("京", &None, None).unwrap();
    let morpheme = morpheme_list.get(0).unwrap();
    assert_eq!(None, morpheme.dictionary_id());
  }

  #[test]
  fn test_tokenize_kanji_alphabet_word() {
    let (_, tokenizer) = &build_tokenizer();
    assert_eq!(1, tokenizer.tokenize("特a", &None, None).unwrap().len());
    assert_eq!(1, tokenizer.tokenize("ab", &None, None).unwrap().len());
    assert_eq!(2, tokenizer.tokenize("特ab", &None, None).unwrap().len());
  }

  #[test]
  fn test_tokenize_multiline_sentences() {
    let (_, tokenizer) = &build_tokenizer();
    let ms = tokenizer
      .tokenize("我輩は猫である。\n名前はまだない。", &None, None)
      .unwrap();
    assert_eq!(17, ms.len());
    assert_eq!(
      vec![
        String::from("名詞"),
        String::from("普通名詞"),
        String::from("一般"),
        String::from("*"),
        String::from("*"),
        String::from("*")
      ],
      ms.get(0).unwrap().part_of_speech()
    )
  }
}
