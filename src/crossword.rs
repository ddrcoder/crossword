extern crate rand;

use ncurses::*;
use rand::{rngs::ThreadRng, Rng};
use std::io::Result;
use std::mem::swap;
use tui::View;
use words::dictionary::{english_scrabble_dict, Dictionary};
use words::{LetterInventory, LetterSet};

#[derive(Clone, Debug, Default)]
struct Line {
  length: usize,
  words: Vec<u32>,
  cells: Vec<u32>,
  inventories: Vec<LetterInventory>,
}

fn intersect(mut a: &[u32], mut b: &[u32]) -> Vec<u32> {
  let (a_in, b_in) = (a, b);
  assert!(!a.is_empty());
  assert!(!b.is_empty());
  if a.len() > b.len() {
    return intersect(b, a);
  }
  let mut ret = vec![];
  while a.len() != 0 {
    let av = a[0];
    a = &a[1..];
    match b.binary_search(&av) {
      Ok(bi) => {
        ret.push(av);
        b = &b[(bi + 1)..];
      }
      Err(bi) => {
        b = &b[bi..];
        if b.is_empty() {
          break;
        }
        while !a.is_empty() && a[0] < b[0] {
          a = &a[1..];
        }
      }
    }
  }
  assert!(!ret.is_empty(), "{:?} & {:?}", a_in, b_in);
  ret
}

#[cfg(test)]
mod test_intersect {
  use super::intersect;

  #[test]
  fn basic() {
    assert_eq!(
      vec![6, 12],
      intersect(&[2, 4, 6, 8, 10, 12, 14], &[3, 6, 9, 12, 15])
    );
  }
}

impl Line {
  fn reset_inventories(&mut self, dictionary: &Dictionary) {
    for inv in &mut self.inventories {
      *inv = LetterInventory::new();
    }
    assert!(!self.words.is_empty());
    let (words, inventories) = (self.words.iter().cloned(), &mut self.inventories);

    let mut count = 0;
    dictionary.visit_indices(words, |_, str| {
      count += 1;
      for (i, ch) in str.chars().enumerate() {
        inventories[i].add(ch);
      }
    });
    assert_ne!(count, 0);
    for inv in &mut self.inventories {
      assert!(!inv.is_empty());
    }
  }

  fn new(length: usize, words: &[u32]) -> Self {
    Self {
      length: length,
      words: words.iter().cloned().collect(),
      cells: (0..length).map(|_| Default::default()).collect(),
      inventories: (0..length).map(|_| Default::default()).collect(),
    }
  }

  fn subset(&mut self, filter_set: &[u32], dictionary: &Dictionary) -> Line {
    let mut new = Line {
      length: self.length,
      words: intersect(&self.words, filter_set),
      cells: self.cells.clone(),
      inventories: self.inventories.clone(),
    };

    new.reset_inventories(dictionary);
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
struct Choice {
  cell_index: usize,
  save_lines: [Line; 2],
}

#[derive(Default)]
pub struct WordIndices {
  length_words: Vec<Vec<u32>>,
  //x[length][index][char][]
  length_index_char_words: Vec<Vec<Vec<Vec<u32>>>>,
}
fn ind_mut<T: Default>(v: &mut Vec<T>, i: usize) -> &mut T {
  if v.len() <= i {
    v.resize_with(i + 1, Default::default);
  }
  &mut v[i]
}

impl WordIndices {
  pub fn new(dict: &Dictionary) -> Self {
    let mut ret: Self = Default::default();
    dict.visit_all(|wi, str| {
      let len = str.len();
      ind_mut(&mut ret.length_words, len).push(wi);
      let index_char_words = ind_mut(&mut ret.length_index_char_words, len);
      index_char_words.resize_with(len, Default::default);
      for (char_words, ch) in index_char_words.iter_mut().zip(str.chars()) {
        if let Some(li) = LetterSet::index(ch) {
          ind_mut(char_words, li as usize).push(wi);
        }
      }
    });
    ret
  }

  fn max_length(&self) -> usize {
    self.length_words.len() - 1
  }

  fn with_length(&self, length: usize) -> &[u32] {
    &self.length_words[length][..]
  }

  fn with_length_char_at(&self, length: usize, ch: char, index: usize) -> Option<&[u32]> {
    if let Some(li) = LetterSet::index(ch) {
      Some(
        &self
          .length_index_char_words
          .get(length)?
          .get(index)?
          .get(li as usize)?[..],
      )
    } else {
      None
    }
  }
}
pub struct Crossword {
  dictionary: Dictionary,
  word_indices: WordIndices,
  width: usize,
  height: usize,
  lines: Vec<Line>,
  cells: Vec<Cell>,
  choices: Vec<Choice>,
}

enum Choices {
  Failure,
  Success,
  Single(usize, char),
  Many(usize, Vec<char>),
}

impl Crossword {
  pub fn new(width: usize, height: usize) -> Result<Self> {
    let dictionary = english_scrabble_dict()?;
    let word_indices = WordIndices::new(&dictionary);
    let line_inits: Vec<Line> = (0..word_indices.max_length())
      .map(|len| Line::new(len, word_indices.with_length(len)))
      .collect();
    let mut cells: Vec<Cell> = vec![];
    for i in 0..height {
      for j in 0..width {
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
      let mut line = line_inits[width].clone();
      for (j, cell) in line.cells.iter_mut().enumerate() {
        let ci = cell_index(i, j);
        *cell = ci as u32;
        cells[ci].lines[0] = (lines.len() as u32, j as u8);
      }
      lines.push(line)
    }

    for j in 0..width {
      let mut line = line_inits[height].clone();
      for (i, cell) in line.cells.iter_mut().enumerate() {
        let ci = cell_index(i, j);
        *cell = ci as u32;
        cells[ci].lines[1] = (lines.len() as u32, i as u8);
      }
      lines.push(line)
    }
    for line in &mut lines[..] {
      line.reset_inventories(&dictionary);
    }
    let mut ret = Self {
      dictionary,
      word_indices,
      width,
      height,
      lines,
      cells,
      choices: vec![],
    };
    for i in 0..ret.cells.len() {
      ret.update_cell(i);
    }
    Ok(ret)
  }

  pub fn update_cell(&mut self, index: usize) {
    let c = &mut self.cells[index];
    let a = &self.lines[c.lines[0].0 as usize].inventories[c.lines[0].1 as usize];
    let b = &self.lines[c.lines[1].0 as usize].inventories[c.lines[1].1 as usize];
    let prod = LetterInventory::product(a, b);
    c.char_dist = prod;
  }

  fn get_next_choices(&self, rng: &mut ThreadRng) -> Choices {
    let cell_index = (0..self.cells.len())
      .filter(|&i| self.cells[i].choice.is_none())
      .min_by_key(|&i| self.cells[i].char_dist.letter_set().len());
    if cell_index.is_none() {
      return Choices::Success;
    }
    let cell_index = cell_index.unwrap();
    let inventory = &self.cells[cell_index].char_dist;
    let set = inventory.letter_set();
    match inventory.letter_set().len() {
      0 => Choices::Failure,
      1 => Choices::Single(cell_index, set.chars().next().unwrap()),
      _ => {
        let mut to_draw: Vec<(char, f32)> = inventory
          .entries()
          .map(|(ch, n)| (ch, rng.gen::<f32>().ln() / -(n as f32)))
          .collect();
        to_draw.sort_unstable_by(|(_, t1), (_, t2)| t1.partial_cmp(t2).unwrap());
        Choices::Many(cell_index, to_draw.into_iter().map(|(ch, _)| ch).collect())
      }
    }
  }

  pub fn choose(&mut self, cell_index: usize, ch: char) -> bool {
    let cell = &mut self.cells[cell_index];
    if cell.choice != None || cell.char_dist.count(ch) == 0 {
      return false;
    }
    cell.choice = Some(ch);
    let lines = cell.lines;
    let mut choice = Choice {
      cell_index,
      ..Default::default()
    };
    for (slot, (index, offset)) in lines[..].iter().cloned().enumerate() {
      let line = &mut self.lines[index as usize];

      choice.save_lines[slot] = line.subset(
        self
          .word_indices
          .with_length_char_at(line.length, ch, offset as usize)
          .unwrap(),
        &self.dictionary,
      );
      let cells = self.lines[index as usize].cells.clone();
      for lc in cells {
        self.update_cell(lc as usize);
      }
    }

    self.choices.push(choice);
    true
  }

  fn undo_one(&mut self) -> bool {
    if let Some(choice) = self.choices.pop() {
      let cell_index = choice.cell_index;
      let cell = &mut self.cells[cell_index];
      mv(cell.row as i32, cell.col as i32);
      addch('_' as u32);
      cell.choice = None;
      let lines = cell.lines;
      self.lines[lines[0].0 as usize] = choice.save_lines[0].clone();
      self.lines[lines[1].0 as usize] = choice.save_lines[1].clone();
      for (index, _) in &lines[..] {
        let cells = self.lines[*index as usize].cells.clone();
        for lc in cells {
          self.update_cell(lc as usize);
        }
      }
      true
    } else {
      false
    }
  }

  fn rec(&mut self, c: &mut usize, rng: &mut ThreadRng) -> bool {
    match self.get_next_choices(rng) {
      Choices::Failure => {
        *c += 1;
        self.render(0, 0);
        refresh();
        if *c % 0x1000 == 0 {}
        false
      }
      Choices::Success => true,
      Choices::Single(cell_index, ch) => {
        assert!(self.choose(cell_index, ch));
        if self.rec(c, rng) {
          return true;
        }
        *c += 1;
        self.undo_one();
        false
      }
      Choices::Many(cell_index, chars) => {
        for ch in chars {
          assert!(self.choose(cell_index, ch));
          if self.rec(c, rng) {
            return true;
          }
          self.undo_one();
        }
        false
      }
    }
  }
}

impl View for Crossword {
  fn cursor(&self, x: i32, y: i32) {
    mv(y, x);
  }
  fn interact(&mut self) {
    let (mut x, mut y) = (0, 0);
    let mut rng = rand::thread_rng();
    let mut downward = false;
    loop {
      x = x.clamp(0, self.width as i32 - 1);
      y = y.clamp(0, self.height as i32 - 1);
      self.render(0, 0);
      if self.choices.len() == self.height * self.width {
        self.cursor(self.width as i32, self.height as i32);
      } else {
        self.cursor(x, y);
      }
      let message: Option<String> = match getch() as u8 {
        0x9 => {
          downward = !downward;
          None
        }
        0x7f => {
          // backspace
          if let Some(choice) = self.choices.last() {
            let cell = &self.cells[choice.cell_index];
            x = cell.col as i32;
            y = cell.row as i32;
            self.undo_one();
          }
          None
        }
        ch @ (0x61..=0x79) => {
          let cell_index = (0..self.cells.len())
            .filter(|ci| {
              let cell = &self.cells[*ci];
              (cell.col as i32, cell.row as i32) == (x, y)
            })
            .next()
            .unwrap();
          let ch = ch as char;
          let ret = if let Some(existing) = self.cells[cell_index].choice {
            if existing == ch {
              None
            } else {
              Some(format!("Already filled (backspace it)"))
            }
          } else if self.choose(cell_index, ch) {
            None
          } else {
            Some(format!("That's a dead end."))
          };
          if ret.is_none() {
            if downward {
              y += 1;
            } else {
              x += 1;
            }
          }
          ret
        }
        0x20 => {
          let mut c = 0;
          Some(if self.rec(&mut c, &mut rng) {
            mv(self.height as i32 + 1, 0);
            format!("Solved in {} steps", c)
          } else {
            format!("Unsolvable, tried {} steps", c)
          })
        }
        //left
        0x44 => {
          x -= 1;
          None
        }
        //right
        0x43 => {
          x += 1;
          None
        }
        0x5b => None,
        //up
        0x41 => {
          y -= 1;
          None
        }
        //down
        0x42 => {
          y += 1;
          None
        }
        //escape
        //0x1b => while self.undo_one() {},
        0x1b => None,
        0x2c => Some(format!("Narrower")),
        0x2e => Some(format!("Wider")),
        0x2d => Some(format!("Shorter")),
        0x3d => Some(format!("Taller")),
        c => Some(format!("Unrecognized char: {:x}", c)),
      };
      self.cursor(0, self.height as i32 + 2);
      if let Some(s) = message {
        addstr(&s);
      }
      addstr("                         ");
      refresh();
    }
  }

  fn render(&self, x: i32, y: i32) {
    let mut height = 0;
    getmaxyx(stdscr(), &mut height, &mut 0);
    for cell in &self.cells {
      mv(y + cell.row as i32, x + cell.col as i32);
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
