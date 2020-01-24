use std::io::{Cursor, Seek, Write};

use log::info;

use super::dictionary_builder::{DictionaryBuilder, DictionaryBuilderErr, WordIdToIdConverter};
use super::double_array_lexicon::DoubleArrayLexicon;
use super::grammar::{GetPartOfSpeech, Grammar};
use super::lexicon::GetWordId;

pub struct UserDictionaryBuilder {
  dictionary_builder: DictionaryBuilder,
  grammar: Grammar,
  system_lexicon: DoubleArrayLexicon,
}

impl UserDictionaryBuilder {
  pub fn new(grammar: Grammar, system_lexicon: DoubleArrayLexicon) -> UserDictionaryBuilder {
    UserDictionaryBuilder {
      dictionary_builder: DictionaryBuilder::default(),
      grammar,
      system_lexicon,
    }
  }
  pub fn build<W: Write + Seek>(
    &mut self,
    lexicon_paths: &[&str],
    output_stream: &mut W,
  ) -> Result<(), DictionaryBuilderErr> {
    info!("reading the source file...");
    for path in lexicon_paths {
      self.dictionary_builder.build_lexicons(path)?;
    }
    info!("{} words", self.dictionary_builder.entries.len());

    self
      .dictionary_builder
      .write_grammar::<Cursor<&[u8]>, W>(None, output_stream)?;
    self.dictionary_builder.write_lexicon(output_stream)?;
    Ok(())
  }
}

impl WordIdToIdConverter for UserDictionaryBuilder {
  fn get_pos_id(&self, strs: &[&str]) -> Result<u16, DictionaryBuilderErr> {
    let pos_id = self.grammar.get_part_of_speech_id(strs);
    match pos_id {
      Some(i) => Ok(i as u16),
      None => Ok(
        self.dictionary_builder.get_pos_id(strs)? + self.grammar.get_part_of_speech_size() as u16,
      ),
    }
  }
  fn get_word_id(
    &self,
    headword: &str,
    pos_id: u16,
    reading_form: &str,
  ) -> Result<u32, DictionaryBuilderErr> {
    match self
      .dictionary_builder
      .get_word_id(headword, pos_id, reading_form)
    {
      Ok(wid) => Ok(wid | 1 << 28),
      Err(_) => Ok(
        self
          .system_lexicon
          .get_word_id(headword, pos_id, reading_form)? as u32,
      ),
    }
  }
}
