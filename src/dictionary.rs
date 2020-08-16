use std::ffi::OsStr;
use std::io::Error as IOError;
use std::path::Path;
use std::sync::{Arc, Mutex};

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
  #[error("{0}")]
  IOError(#[from] IOError),
  #[error("{0}")]
  ConfigErr(#[from] ConfigErr),
  #[error("{0}")]
  SudachiDictErr(#[from] SudachiDictErr),
  #[error("{0}")]
  ReadDictionaryErr(#[from] ReadDictionaryErr),
  #[error("{0}")]
  InputTextPluginGetErr(#[from] InputTextPluginGetErr),
  #[error("{0}")]
  OovProviderPluginGetErr(#[from] OovProviderPluginGetErr),
  #[error("{0}")]
  ReadCharacterDefinitionErr(#[from] ReadCharacterDefinitionErr),
}

type InputTextPlugins = Arc<Vec<InputTextPlugin>>;
type OovProviderPlugins = Arc<Vec<OovProviderPlugin>>;
type PathRewritePlugins = Arc<Vec<PathRewritePlugin>>;

pub struct Dictionary {
  grammar: Arc<Mutex<Grammar>>,
  lexicon_set: Arc<Mutex<LexiconSet>>,
  input_text_plugins: InputTextPlugins,
  oov_provider_plugins: OovProviderPlugins,
  path_rewrite_plugins: PathRewritePlugins,
}

impl Dictionary {
  pub fn new(
    grammar: &Arc<Mutex<Grammar>>,
    lexicon_set: &Arc<Mutex<LexiconSet>>,
    input_text_plugins: &InputTextPlugins,
    oov_provider_plugins: &OovProviderPlugins,
    path_rewrite_plugins: &PathRewritePlugins,
  ) -> Dictionary {
    Dictionary {
      grammar: Arc::clone(grammar),
      lexicon_set: Arc::clone(lexicon_set),
      input_text_plugins: Arc::clone(input_text_plugins),
      oov_provider_plugins: Arc::clone(oov_provider_plugins),
      path_rewrite_plugins: Arc::clone(path_rewrite_plugins),
    }
  }
  pub fn get_grammar(&self) -> Arc<Mutex<Grammar>> {
    Arc::clone(&self.grammar)
  }
  pub fn setup(
    config_path: Option<&str>,
    resource_dir: Option<&str>,
    python_exe: Option<&OsStr>,
  ) -> Result<Dictionary, DictionaryErr> {
    let mut config = Config::setup(config_path, resource_dir)?;
    let mut system_dictionary =
      Dictionary::read_system_dictionary(config.system_dict_path(python_exe)?)?;

    let char_category = Dictionary::read_character_definition(config.char_def_path()?)?;
    system_dictionary
      .grammar
      .set_character_category(Some(char_category));

    let lexicon_set = Arc::new(Mutex::new(LexiconSet::new(system_dictionary.lexicon)));
    let grammar = Arc::new(Mutex::new(system_dictionary.grammar));

    let input_text_plugins = Arc::new(get_input_text_plugins(&config)?);

    let oov_provider_plugins = Arc::new(get_oov_provider_plugins(&config, Arc::clone(&grammar))?);

    let path_rewrite_plugins: Vec<PathRewritePlugin> = vec![];
    let path_rewrite_plugins = Arc::new(path_rewrite_plugins);

    for user_dict_path in config.user_dict_paths() {
      let user_dictionary = Dictionary::read_user_dictionary(user_dict_path, &lexicon_set)?;

      let mut user_lexicon = user_dictionary.lexicon;
      let tokenizer = Tokenizer::new(
        Arc::clone(&grammar),
        Arc::clone(&lexicon_set),
        Arc::clone(&input_text_plugins),
        Arc::clone(&oov_provider_plugins),
        Arc::new(vec![]),
      );
      user_lexicon.calculate_cost(&tokenizer);
      lexicon_set.lock().unwrap().add(
        user_lexicon,
        grammar.lock().unwrap().get_part_of_speech_size(),
      );
      grammar
        .lock()
        .unwrap()
        .add_pos_list(&user_dictionary.grammar);
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
      Arc::clone(&self.grammar),
      Arc::clone(&self.lexicon_set),
      Arc::clone(&self.input_text_plugins),
      Arc::clone(&self.oov_provider_plugins),
      Arc::clone(&self.path_rewrite_plugins),
    )
  }

  pub fn read_system_dictionary<P: AsRef<Path>>(
    filename: P,
  ) -> Result<BinaryDictionary, ReadDictionaryErr> {
    BinaryDictionary::from_system_dictionary(filename)
  }

  pub fn read_user_dictionary<P: AsRef<Path>>(
    filename: P,
    lexicon_set: &Arc<Mutex<LexiconSet>>,
  ) -> Result<BinaryDictionary, DictionaryErr> {
    if lexicon_set.lock().unwrap().is_full() {
      return Err(DictionaryErr::TooManyDictionariesErr);
    }
    let user_dictionary = BinaryDictionary::from_user_dictionary(filename)?;
    Ok(user_dictionary)
  }

  pub fn read_character_definition<P: AsRef<Path>>(
    filename: P,
  ) -> Result<CharacterCategory, ReadCharacterDefinitionErr> {
    let char_category = CharacterCategory::read_character_definition(&filename)?;
    Ok(char_category)
  }
}
