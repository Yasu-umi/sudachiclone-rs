use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use log::{info, log_enabled, Level};

use super::dictionary_lib::grammar::Grammar;
use super::dictionary_lib::grammar::INHIBITED_CONNECTION;
use super::lattice_node::LatticeNode;

pub struct Lattice {
  size: usize,
  capacity: usize,
  eos_node: Option<Rc<RefCell<LatticeNode>>>,
  end_lists: Vec<Vec<Rc<RefCell<LatticeNode>>>>,
  grammar: Rc<RefCell<Grammar>>,
  eos_parameters: [u32; 3],
}

impl Lattice {
  pub fn new(grammar: Rc<RefCell<Grammar>>) -> Lattice {
    let eos_parameters = grammar.borrow().get_eos_parameter();
    let bos_params = grammar.borrow().get_bos_parameter();
    let mut bos_node = LatticeNode::empty(bos_params[0], bos_params[1], bos_params[2] as i32);
    bos_node.is_connected_to_bos = true;
    Lattice {
      size: 0,
      capacity: 0,
      eos_node: None,
      end_lists: vec![vec![Rc::new(RefCell::new(bos_node))]],
      grammar,
      eos_parameters,
    }
  }
  pub fn resize(&mut self, size: usize) {
    if size > self.capacity {
      self.expand(size);
    }
    self.size = size;
    let mut eos_node = LatticeNode::empty(
      self.eos_parameters[0],
      self.eos_parameters[1],
      self.eos_parameters[2] as i32,
    );
    eos_node.start = size;
    eos_node.end = size;
    self.eos_node = Some(Rc::new(RefCell::new(eos_node)));
  }
  pub fn clear(&mut self) {
    for node in self.end_lists.iter_mut() {
      node.clear();
    }
    self.size = 0;
    self.eos_node = None;
  }
  fn expand(&mut self, new_size: usize) {
    let expand_list: Vec<Vec<Rc<RefCell<LatticeNode>>>> = vec![vec![]; new_size - self.size];
    self.end_lists.extend(expand_list);
    self.capacity = new_size;
  }
  pub fn insert(&mut self, start: usize, end: usize, node: Rc<RefCell<LatticeNode>>) {
    let mut _node = node.borrow_mut();
    _node.start = start;
    _node.end = end;
    self.connect_node(_node);
    self.end_lists[end].push(node);
  }
  pub fn has_previous_node(&self, index: usize) -> bool {
    !self.end_lists[index].is_empty()
  }
  fn connect_node(&self, mut r_node: RefMut<LatticeNode>) {
    let start = r_node.start;
    let grammar = self.grammar.borrow();
    r_node.total_cost = i16::max_value() as i32;
    for l_node in self.end_lists[start].iter() {
      let _l_node = l_node.borrow();
      if !_l_node.is_connected_to_bos {
        continue;
      }
      // right_id and left_id look reversed, but it works ...
      let connect_cost =
        grammar.get_connect_cost(_l_node.right_id as usize, r_node.left_id as usize);
      if connect_cost == INHIBITED_CONNECTION {
        continue;
      }
      let cost = _l_node.total_cost + connect_cost as i32;
      if cost < r_node.total_cost {
        r_node.total_cost = cost;
        r_node.best_previous_node = Some(Rc::clone(l_node));
      }
    }
    r_node.is_connected_to_bos = r_node.best_previous_node.is_some();
    r_node.total_cost += r_node.cost;
  }
  pub fn get_best_path(&self) -> Vec<Rc<RefCell<LatticeNode>>> {
    // self.connect_node(self.eos_node);
    let mut result = vec![];
    let eos_node = self.eos_node.as_ref().unwrap().borrow();
    let mut node = eos_node.best_previous_node.clone();
    while {
      if let Some(n) = node.as_ref() {
        n.borrow().id != self.end_lists[0][0].borrow().id
      } else {
        false
      }
    } {
      let n = node.unwrap();
      result.push(Rc::clone(&n));
      node = n.borrow().best_previous_node.clone();
    }
    result.reverse();
    result
  }
  pub fn connect_eos_node(&mut self) {
    self.connect_node(self.eos_node.as_ref().unwrap().borrow_mut());
  }
  fn log_node(&self, node: Ref<LatticeNode>, index: &mut usize) {
    let grammar = self.grammar.borrow();
    let mut surface = String::from("(null)");
    let mut pos = String::from("BOS/EOS");
    if node.is_defined {
      let word_info = node.get_word_info();
      surface = word_info.surface;
      pos = String::from("(null)");
      let pos_id = word_info.pos_id;
      if pos_id >= 0 {
        pos = grammar.get_part_of_speech_string(pos_id as usize).join(",");
      }
    }
    let mut costs = vec![];
    for l_node in self.end_lists[node.start].iter() {
      let cost = grammar.get_connect_cost(l_node.borrow().right_id as usize, node.left_id as usize);
      costs.push(format!("{}", cost));
    }
    info!(
      "{}: {} {} {}({}) {} {} {} {}: {}",
      index,
      node.get_start(),
      node.get_end(),
      surface,
      node.word_id,
      pos,
      node.left_id,
      node.right_id,
      node.cost,
      costs.join(" "),
    );
    *index += 1;
  }
  pub fn log(&self) {
    if !log_enabled!(Level::Info) {
      return;
    }
    let mut index = 0;
    for i in 0..=(self.size + 1) {
      let i = self.size + 1 - i;
      if i <= self.size {
        for r_node in self.end_lists[i].iter() {
          self.log_node(r_node.borrow(), &mut index);
        }
      } else {
        self.log_node(self.eos_node.as_ref().unwrap().borrow(), &mut index);
      }
    }
  }
}
