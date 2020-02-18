use std::cell::RefCell;
use std::io::Error as IOError;
use std::path::Path;
use std::rc::Rc;

use thiserror::Error;

use super::config::{Config, ConfigErr, SudachiDictErr};
use super::dictionary_lib::binary_dictionary::{BinaryDictionary, ReadDictionaryErr};
use super::dictionary_lib::character_category::{CharacterCategory, ReadCharacterDefinitionErr};
use super::dictionary_lib::grammar::{Grammar, SetCharacterCategory};
use super::dictionary_lib::lexicon_set::LexiconSet;
use super::plugin::input_text_plugin::{
  get_input_text_plugins, InputTextPlugin, InputTextPluginGetErr,
};
use super::plugin::oov_provider_plugin::{
  get_oov_provider_plugins, OovProviderPlugin, OovProviderPluginGetErr,
};
use super::plugin::path_rewrite_plugin::PathRewritePlugin;
use super::tokenizer::Tokenizer;

#[derive(Error, Debug)]
pub enum DictionaryErr {
  #[error("too many dictionaries")]
  TooManyDictionariesErr,
  #[error("{self:?}")]
  IOError(#[from] IOError),
  #[error("{self:?}")]
  ConfigErr(#[from] ConfigErr),
  #[error("{self:?}")]
  SudachiDictErr(#[from] SudachiDictErr),
  #[error("{self:?}")]
  ReadDictionaryErr(#[from] ReadDictionaryErr),
  #[error("{self:?}")]
  InputTextPluginGetErr(#[from] InputTextPluginGetErr),
  #[error("{self:?}")]
  OovProviderPluginGetErr(#[from] OovProviderPluginGetErr),
  #[error("{self:?}")]
  ReadCharacterDefinitionErr(#[from] ReadCharacterDefinitionErr),
}

type InputTextPlugins = Rc<Vec<Box<dyn InputTextPlugin>>>;
type OovProviderPlugins = Rc<Vec<Box<dyn OovProviderPlugin>>>;
type PathRewritePlugins = Rc<Vec<Box<dyn PathRewritePlugin>>>;

pub struct Dictionary {
  grammar: Rc<RefCell<Grammar>>,
  lexicon_set: Rc<RefCell<LexiconSet>>,
  input_text_plugins: InputTextPlugins,
  oov_provider_plugins: OovProviderPlugins,
  path_rewrite_plugins: PathRewritePlugins,
}

impl Dictionary {
  pub fn new(
    grammar: &Rc<RefCell<Grammar>>,
    lexicon_set: &Rc<RefCell<LexiconSet>>,
    input_text_plugins: &InputTextPlugins,
    oov_provider_plugins: &OovProviderPlugins,
    path_rewrite_plugins: &PathRewritePlugins,
  ) -> Dictionary {
    Dictionary {
      grammar: Rc::clone(grammar),
      lexicon_set: Rc::clone(lexicon_set),
      input_text_plugins: Rc::clone(input_text_plugins),
      oov_provider_plugins: Rc::clone(oov_provider_plugins),
      path_rewrite_plugins: Rc::clone(path_rewrite_plugins),
    }
  }
  pub fn get_grammar(&self) -> Rc<RefCell<Grammar>> {
    Rc::clone(&self.grammar)
  }
  pub fn setup(
    config_path: Option<&str>,
    resource_dir: Option<&str>,
  ) -> Result<Dictionary, DictionaryErr> {
    let mut config = Config::setup(config_path, resource_dir)?;
    let mut system_dictionary = Dictionary::read_system_dictionary(config.system_dict_path()?)?;

    let char_category = Dictionary::read_character_definition(config.char_def_path()?)?;
    system_dictionary
      .grammar
      .set_character_category(Some(char_category));

    let lexicon_set = Rc::new(RefCell::new(LexiconSet::new(system_dictionary.lexicon)));
    let grammar = Rc::new(RefCell::new(system_dictionary.grammar));

    let input_text_plugins = Rc::new(get_input_text_plugins(&config)?);

    let oov_provider_plugins = Rc::new(get_oov_provider_plugins(&config, Rc::clone(&grammar))?);

    let path_rewrite_plugins: Vec<Box<dyn PathRewritePlugin>> = vec![];
    let path_rewrite_plugins = Rc::new(path_rewrite_plugins);

    for user_dict_path in config.user_dict_paths() {
      let user_dictionary = Dictionary::read_user_dictionary(user_dict_path, &lexicon_set)?;

      let mut user_lexicon = user_dictionary.lexicon;
      let tokenizer = Tokenizer::new(
        Rc::clone(&grammar),
        Rc::clone(&lexicon_set),
        Rc::clone(&input_text_plugins),
        Rc::clone(&oov_provider_plugins),
        Rc::new(vec![]),
      );
      user_lexicon.calculate_cost(&tokenizer);
      lexicon_set
        .borrow_mut()
        .add(user_lexicon, grammar.borrow().get_part_of_speech_size());
      grammar.borrow_mut().add_pos_list(&user_dictionary.grammar);
    }

    Ok(Dictionary::new(
      &grammar,
      &lexicon_set,
      &input_text_plugins,
      &oov_provider_plugins,
      &path_rewrite_plugins,
    ))
  }

  pub fn create(&self) -> Tokenizer {
    Tokenizer::new(
      Rc::clone(&self.grammar),
      Rc::clone(&self.lexicon_set),
      Rc::clone(&self.input_text_plugins),
      Rc::clone(&self.oov_provider_plugins),
      Rc::clone(&self.path_rewrite_plugins),
    )
  }

  pub fn read_system_dictionary<P: AsRef<Path>>(
    filename: P,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    BinaryDictionary::from_system_dictionary(filename)
  }

  pub fn read_user_dictionary<P: AsRef<Path>>(
    filename: P,
    lexicon_set: &Rc<RefCell<LexiconSet>>,
  ) -> Result<BinaryDictionary, DictionaryErr> {
    if lexicon_set.borrow().is_full() {
      return Err(DictionaryErr::TooManyDictionariesErr);
    }
    let user_dictionary = BinaryDictionary::from_user_dictionary(filename)?;
    Ok(user_dictionary)
  }

  pub fn read_character_definition<P: AsRef<Path>>(
    filename: P,
  ) -> Result<CharacterCategory, ReadCharacterDefinitionErr> {
    let mut char_category = CharacterCategory::default();
    char_category.read_character_definition(&filename)?;
    Ok(char_category)
  }
}
