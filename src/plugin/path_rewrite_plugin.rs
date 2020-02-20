use std::sync::{Arc, Mutex};

use crate::lattice::Lattice;
use crate::lattice_node::LatticeNode;
use crate::utf8_input_text::UTF8InputText;

pub enum PathRewritePlugin {}

pub trait RewritePath {
  fn rewrite(&self, text: &UTF8InputText, path: &[Arc<Mutex<LatticeNode>>], lattice: &Lattice);
}

impl RewritePath for PathRewritePlugin {
  fn rewrite(&self, _text: &UTF8InputText, _path: &[Arc<Mutex<LatticeNode>>], _lattice: &Lattice) {}
}
