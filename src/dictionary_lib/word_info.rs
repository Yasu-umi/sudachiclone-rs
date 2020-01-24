#[derive(Clone, Debug)]
pub struct WordInfo {
  pub surface: String,
  pub head_word_length: usize,
  pub pos_id: i16,
  pub normalized_form: String,
  pub dictionary_form_word_id: i32,
  pub dictionary_form: String,
  pub reading_form: String,
  pub a_unit_split: Vec<i32>,
  pub b_unit_split: Vec<i32>,
  pub word_structure: Vec<i32>,
}
