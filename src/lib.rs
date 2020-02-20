//! ```
//! use sudachiclone::prelude::*;
//!
//! let dictionary = Dictionary::setup(None, None).unwrap();
//! let tokenizer = dictionary.create();
//!
//! // Multi-granular tokenization
//! // using `system_core.dic` or `system_full.dic` version 20190781
//! // you may not be able to replicate this particular example due to dictionary you use
//!
//! for m in tokenizer.tokenize("国家公務員", &Some(SplitMode::C), None).unwrap() {
//!     println!("{}", m.surface());
//! };
//! // => 国家公務員
//!
//! for m in tokenizer.tokenize("国家公務員", &Some(SplitMode::B), None).unwrap() {
//!     println!("{}", m.surface());
//! };
//! // => 国家
//! // => 公務員
//!
//! for m in tokenizer.tokenize("国家公務員", &Some(SplitMode::A), None).unwrap() {
//!     println!("{}", m.surface());
//! };
//! // => 国家
//! // => 公務
//! // => 員
//!
//! // Morpheme information
//!
//! let m = tokenizer.tokenize("食べ", &Some(SplitMode::A), None).unwrap().get(0).unwrap();
//! println!("{}", m.surface());
//! // => 食べ
//! println!("{}", m.dictionary_form());
//! // => 食べる
//! println!("{}", m.reading_form());
//! // => タベ
//! println!("{:?}", m.part_of_speech());
//! // => ["動詞", "一般", "*", "*", "下一段-バ行", "連用形-一般"]
//!
//! // Normalization
//!
//! println!("{}", tokenizer.tokenize("附属", &Some(SplitMode::A), None).unwrap().get(0).unwrap().normalized_form());
//! // => 付属
//!
//! println!("{}", tokenizer.tokenize("SUMMER", &Some(SplitMode::A), None).unwrap().get(0).unwrap().normalized_form());
//! // => サマー
//!
//! println!("{}", tokenizer.tokenize("シュミレーション", &Some(SplitMode::A), None).unwrap().get(0).unwrap().normalized_form());
//! // => シミュレーション
//! ```

#![crate_name = "sudachiclone"]
#![crate_type = "lib"]
#![crate_type = "dylib"]
#![crate_type = "rlib"]

pub mod config;
pub mod darts;
pub mod dictionary;
pub mod dictionary_lib;
pub mod lattice;
pub mod lattice_node;
pub mod morpheme;
pub mod morpheme_list;
pub mod plugin;
mod resources;
pub mod tokenizer;
pub mod utf8_input_text;
pub mod utf8_input_text_builder;

pub mod prelude {
  pub use crate::dictionary::Dictionary;
  pub use crate::tokenizer::{CanTokenize, SplitMode, Tokenizer};
}
