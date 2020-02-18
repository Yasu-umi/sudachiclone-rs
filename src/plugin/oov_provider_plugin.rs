use std::sync::{Arc, Mutex};

use serde_json::Value;
use thiserror::Error;

use super::mecab_oov_plugin::{MecabOovPlugin, MecabOovPluginSetupErr};
use super::simple_oov_plugin::{SimpleOovPlugin, SimpleOovPluginSetupErr};
use crate::config::Config;
use crate::dictionary_lib::grammar::Grammar;
use crate::lattice_node::LatticeNode;
use crate::utf8_input_text::{InputText, UTF8InputText};

pub enum OovProviderPlugin {
  MecabOovPlugin(MecabOovPlugin),
  SimpleOovPlugin(SimpleOovPlugin),
}

pub trait ProvideOov<T: InputText = UTF8InputText> {
  fn provide_oov(
    &self,
    input_text: &T,
    offset: usize,
    has_other_words: bool,
  ) -> Vec<Arc<Mutex<LatticeNode>>>;
}

impl<T: InputText> ProvideOov<T> for OovProviderPlugin {
  fn provide_oov(
    &self,
    input_text: &T,
    offset: usize,
    has_other_words: bool,
  ) -> Vec<Arc<Mutex<LatticeNode>>> {
    match self {
      OovProviderPlugin::MecabOovPlugin(plugin) => {
        plugin.provide_oov(input_text, offset, has_other_words)
      }
      OovProviderPlugin::SimpleOovPlugin(plugin) => {
        plugin.provide_oov(input_text, offset, has_other_words)
      }
    }
  }
}

pub fn get_oov<T: InputText>(
  plugin: &OovProviderPlugin,
  input_text: &T,
  offset: usize,
  has_other_words: bool,
) -> Vec<Arc<Mutex<LatticeNode>>> {
  let nodes = plugin.provide_oov(input_text, offset, has_other_words);
  for node in nodes.iter() {
    let mut node = node.lock().unwrap();
    node.start = offset;
    node.end = offset + node.get_word_info().head_word_length;
  }
  nodes
}

#[derive(Error, Debug)]
pub enum OovProviderPluginGetErr {
  #[error("{0} is invalid InputTextPlugin class")]
  InvalidClassErr(String),
  #[error("config file is invalid format")]
  InvalidFormatErr,
  #[error("{self:?}")]
  MecabOovPluginSetupErr(#[from] MecabOovPluginSetupErr),
  #[error("{self:?}")]
  SimpleOovPluginSetupErr(#[from] SimpleOovPluginSetupErr),
}

fn get_oov_provider_plugin(
  config: &Config,
  json_obj: &Value,
  grammar: Arc<Mutex<Grammar>>,
) -> Result<OovProviderPlugin, OovProviderPluginGetErr> {
  if let Some(Value::String(class)) = json_obj.get("class") {
    if class == "sudachipy.plugin.oov.SimpleOovProviderPlugin" {
      Ok(OovProviderPlugin::SimpleOovPlugin(SimpleOovPlugin::setup(
        json_obj, grammar,
      )?))
    } else if class == "sudachipy.plugin.oov.MeCabOovProviderPlugin" {
      Ok(OovProviderPlugin::MecabOovPlugin(MecabOovPlugin::setup(
        &config.resource_dir,
        json_obj,
        grammar,
      )?))
    } else {
      Err(OovProviderPluginGetErr::InvalidClassErr(class.to_string()))
    }
  } else {
    Err(OovProviderPluginGetErr::InvalidFormatErr)
  }
}

pub fn get_oov_provider_plugins(
  config: &Config,
  grammar: Arc<Mutex<Grammar>>,
) -> Result<Vec<OovProviderPlugin>, OovProviderPluginGetErr> {
  let mut plugins = vec![];
  if let Some(Value::Array(arr)) = config.settings.get("oovProviderPlugin") {
    for v in arr {
      plugins.push(get_oov_provider_plugin(config, v, Arc::clone(&grammar))?);
    }
  }
  Ok(plugins)
}
