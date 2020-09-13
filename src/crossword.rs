#![feature(iter_partition_in_place)]
extern crate rand;

use ncurses::*;
use rand::distributions::{Distribution, Uniform, WeightedIndex};
use std::cmp::max;
use std::mem::swap;
use tui::View;
use words::{LetterInventory, LetterSet};

#[derive(Clone, Debug, Default)]
struct Line<'a> {
  length: usize,
  words: Vec<&'a str>,
  cells: Vec<u32>,
  inventories: Vec<LetterInventory>,
}

impl<'a> Line<'a> {
  fn reset_inventories(&mut self) {
    for inv in &mut self.inventories {
      *inv = LetterInventory::new();
    }
    for word in &self.words {
      assert_eq!(word.len(), self.length);
      for (i, ch) in word.chars().enumerate() {
        self.inventories[i].add(ch);
      }
    }
  }

  fn new(length: usize, words: Vec<&'a str>) -> Self {
    assert_ne!(words.len(), 0);
    let mut ret = Self {
      length: length,
      words: words,
      cells: (0..length).map(|_| Default::default()).collect(),
      inventories: (0..length).map(|_| Default::default()).collect(),
    };
    ret.reset_inventories();
    ret
  }

  fn choose(&mut self, index: u8, ch: char) -> Line<'a> {
    let mut new = Line {
      length: self.length,
      words: self
        .words
        .iter()
        .cloned()
        .filter(|w| w.chars().skip(index as usize).next() == Some(ch.to_ascii_uppercase()))
        .collect(),
      cells: self.cells.clone(),
      inventories: self.inventories.clone(),
    };

    new.reset_inventories();
    swap(&mut new, self);
    new
  }
}

#[derive(Default)]
struct Cell {
  row: usize,
  col: usize,
  lines: [(u32, u8); 2],
  choice: Option<char>,
  char_dist: LetterInventory,
}

#[derive(Default)]
struct Choice<'a> {
  cell_index: usize,
  save_lines: [Line<'a>; 2],
}

pub struct Crossword<'a> {
  width: usize,
  height: usize,
  lines: Vec<Line<'a>>,
  cells: Vec<Cell>,
  choices: Vec<Choice<'a>>,
}

impl<'a> Crossword<'a> {
  pub fn new(width: usize, height: usize, words: &[&'a str]) -> Self {
    assert_ne!(words.len(), 0);
    let mut length_buckets: Vec<Option<Vec<&'a str>>> = vec![];
    length_buckets.resize(max(width, height), None);
    length_buckets[width - 1] = Some(vec![]);
    length_buckets[height - 1] = Some(vec![]);
    for word in words {
      let length = word.len();
      assert_ne!(length, 0);
      if length > length_buckets.len() {
        continue;
      }
      if let &mut Some(ref mut v) = &mut length_buckets[word.len() - 1] {
        v.push(word);
      }
    }
    if let &Some(ref v) = &length_buckets[3] {
      assert_ne!(v.len(), 0);
    }
    let line_inits: Vec<Option<Line<'a>>> = length_buckets
      .into_iter()
      .enumerate()
      .map(|(i, b)| b.map(|words| Line::new(i + 1, words)))
      .collect();
    let mut cells: Vec<Cell> = vec![];
    for i in 0..height {
      for j in 0..width {
        let cell_index = cells.len();
        cells.push(Cell {
          row: i,
          col: j,
          ..Default::default()
        });
      }
    }
    let cell_index = |i, j| i * width + j;
    let mut lines = vec![];
    for i in 0..height {
      let mut line = line_inits[width - 1].clone().unwrap();
      for (j, cell) in line.cells.iter_mut().enumerate() {
        let ci = cell_index(i, j);
        *cell = ci as u32;
        cells[ci].lines[0] = (lines.len() as u32, j as u8);
      }
      lines.push(line)
    }

    for j in 0..width {
      let mut line = line_inits[height - 1].clone().unwrap();
      for (i, cell) in line.cells.iter_mut().enumerate() {
        let ci = cell_index(i, j);
        *cell = ci as u32;
        cells[ci].lines[1] = (lines.len() as u32, i as u8);
      }
      lines.push(line)
    }
    let mut ret = Self {
      width,
      height,
      lines,
      cells,
      choices: vec![],
    };
    for i in 0..ret.cells.len() {
      ret.update_cell(i);
    }
    ret
  }

  pub fn update_cell(&mut self, index: usize) {
    let c = &mut self.cells[index];
    let a = &self.lines[c.lines[0].0 as usize].inventories[c.lines[0].1 as usize];
    let b = &self.lines[c.lines[1].0 as usize].inventories[c.lines[1].1 as usize];
    let prod = LetterInventory::product(a, b);
    c.char_dist = prod;
  }

  pub fn choose_one(&mut self, used_letters: &mut LetterSet) -> bool {
    let cell_index = (0..self.cells.len())
      .filter(|&i| self.cells[i].choice.is_none())
      .min_by_key(|&i| self.cells[i].char_dist.letter_set().len())
      .unwrap();
    let inventory = &self.cells[cell_index].char_dist;
    let mut rng = rand::thread_rng();
    let set = LetterSet::difference(inventory.letter_set(), used_letters);
    if set.len() == 0 {
      return false;
    }
    let index = WeightedIndex::new(set.chars().map(|ch| inventory.count(ch)))
      .unwrap()
      .sample(&mut rng);
    let ch = set.chars().skip(index).next().unwrap();
    used_letters.insert(ch);
    let cell = &mut self.cells[cell_index];
    cell.choice = Some(ch);
    mv(cell.row as i32, cell.col as i32);
    addch(ch as u32);
    let lines = cell.lines;
    let mut choice = Choice {
      cell_index,
      ..Default::default()
    };
    for (slot, (index, offset)) in lines[..].iter().cloned().enumerate() {
      let line = &mut self.lines[index as usize];

      choice.save_lines[slot] = line.choose(offset, ch);
      let cells = self.lines[index as usize].cells.clone();
      for lc in cells {
        self.update_cell(lc as usize);
      }
    }

    self.choices.push(choice);
    true
  }

  fn undo_one(&mut self) {
    if let Some(choice) = self.choices.pop() {
      let cell_index = choice.cell_index;
      let cell = &mut self.cells[cell_index];
      mv(cell.row as i32, cell.col as i32);
      addch('_' as u32);
      cell.choice = None;
      let lines = cell.lines;
      self.lines[lines[0].0 as usize] = choice.save_lines[0].clone();
      self.lines[lines[1].0 as usize] = choice.save_lines[1].clone();
      for (slot, (index, offset)) in lines[..].iter().cloned().enumerate() {
        let cells = self.lines[index as usize].cells.clone();
        for lc in cells {
          self.update_cell(lc as usize);
        }
      }
    }
  }

  fn rec(&mut self, c: &mut usize) -> bool {
    if self.choices.len() == self.width * self.height {
      return true;
    }
    let mut used = LetterSet::new();
    while self.choose_one(&mut used) {
      *c += 1;
      if *c % 10000 == 0 {
        refresh();
      }
      if self.rec(c) {
        return true;
      }
      self.undo_one();
    }
    false
  }

  fn search(&self) -> Vec<String> {
    // Recursively choose letters by picking a cell (how?), then picking a
    // letter from the *joint distribution* of the letters possible in that
    // cell. That is, the product of the two letter inventories.
    vec![]
  }
}

impl<'a> View for Crossword<'a> {
  fn interact(&mut self) {
    loop {
      self.render(0, 0);
      self.cursor(0, 0);
      match getch() {
        0x7f => {
          self.undo_one();
        }
        0x20 => {
          let mut i = 0;
          self.rec(&mut i);
        }
        _ => {
          //self.choose_one();
        }
      }
    }
  }
  fn render(&self, x: i32, mut y: i32) {
    let mut height = 0;
    getmaxyx(stdscr(), &mut height, &mut 0);
    let limit = (height - 3) as usize;
    let before = time::precise_time_ns();
    let matches = self.search();
    let after = time::precise_time_ns();
    for cell in &self.cells {
      mv(cell.row as i32, cell.col as i32);
      match cell.choice {
        Some(ch) => {
          addch(ch as u32);
        }
        None => {
          addch('_' as u32);
        }
      }
    }
  }
}
