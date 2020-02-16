use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;
use thiserror::Error;

use super::mecab_oov_plugin::{MecabOovPlugin, MecabOovPluginSetupErr};
use super::simple_oov_plugin::SimpleOovPlugin;
use crate::config::Config;
use crate::dictionary_lib::grammar::Grammar;
use crate::lattice_node::LatticeNode;
use crate::utf8_input_text::{InputText, UTF8InputText};

#[derive(Error, Debug)]
pub enum OovProviderPluginSetupErr {
  #[error("{self:?}")]
  MecabOovPluginSetupErr(#[from] MecabOovPluginSetupErr),
}

pub trait OovProviderPlugin<T: InputText = UTF8InputText> {
  fn setup(&mut self, grammar: Rc<RefCell<Grammar>>) -> Result<(), OovProviderPluginSetupErr>;
  fn provide_oov(
    &self,
    input_text: &T,
    offset: usize,
    has_other_words: bool,
  ) -> Vec<Rc<RefCell<LatticeNode>>>;
}

pub fn get_oov<T: InputText>(
  plugin: &dyn OovProviderPlugin<T>,
  input_text: &T,
  offset: usize,
  has_other_words: bool,
) -> Vec<Rc<RefCell<LatticeNode>>> {
  let nodes = plugin.provide_oov(input_text, offset, has_other_words);
  for node in nodes.iter() {
    let mut node = node.borrow_mut();
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
}

fn get_oov_provider_plugin(
  config: &Config,
  json_obj: &Value,
) -> Result<Box<dyn OovProviderPlugin>, OovProviderPluginGetErr> {
  if let Some(Value::String(class)) = json_obj.get("class") {
    if class == "sudachipy.plugin.oov.SimpleOovProviderPlugin" {
      Ok(Box::new(SimpleOovPlugin::new(json_obj)))
    } else if class == "sudachipy.plugin.oov.MeCabOovProviderPlugin" {
      Ok(Box::new(MecabOovPlugin::new(
        &config.resource_dir,
        json_obj,
      )))
    } else {
      Err(OovProviderPluginGetErr::InvalidClassErr(class.to_string()))
    }
  } else {
    Err(OovProviderPluginGetErr::InvalidFormatErr)
  }
}

pub fn get_oov_provider_plugins(
  config: &Config,
) -> Result<Vec<Box<dyn OovProviderPlugin<UTF8InputText>>>, OovProviderPluginGetErr> {
  let mut plugins = vec![];
  if let Some(Value::Array(arr)) = config.settings.get("oovProviderPlugin") {
    for v in arr {
      plugins.push(get_oov_provider_plugin(config, v)?);
    }
  }
  Ok(plugins)
}
