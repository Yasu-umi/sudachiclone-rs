use std::cell::RefCell;
use std::rc::Rc;

use crate::dictionary_lib::grammar::Grammar;
use crate::lattice::Lattice;
use crate::lattice_node::LatticeNode;
use crate::utf8_input_text::UTF8InputText;

pub trait PathRewritePlugin {
  fn setup(&mut self, grammar: Rc<RefCell<Grammar>>);
  fn rewrite(&self, text: &UTF8InputText, path: &[Rc<RefCell<LatticeNode>>], lattice: &Lattice);
}
