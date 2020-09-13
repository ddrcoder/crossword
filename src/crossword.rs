#![feature(iter_partition_in_place)]
extern crate rand;

use ncurses::*;
use rand::distributions::{Distribution, Uniform};
use std::cmp::max;
use tui::View;
use words::{LetterInventory, LetterSet};

#[derive(Clone, Default, Debug)]
struct LineCell {
  cell_index: usize,
  inventory: LetterInventory,
}

#[derive(Clone, Debug)]
struct Line<'a> {
  length: usize,
  // histogram
  active_words: usize,
  words: Vec<&'a str>,
  cells: Vec<LineCell>,
}

impl<'a> Line<'a> {
  fn reset_inventories(&mut self) {
    for cell in &mut self.cells {
      cell.inventory = LetterInventory::new();
    }
    for word in &self.words[0..self.active_words] {
      assert_eq!(word.len(), self.length);
      for (i, ch) in word.chars().enumerate() {
        self.cells[i].inventory.add(ch);
        assert_ne!(self.cells[i].inventory.letter_set().len(), 0, "{}", i);
      }
    }
    for (i, cell) in self.cells.iter().enumerate() {
      assert_ne!(
        cell.inventory.letter_set().len(),
        0,
        "{} in {:?} for {:?}",
        i,
        self.cells,
        &self.words[0..self.active_words],
      );
    }
  }

  fn new(length: usize, words: Vec<&'a str>) -> Self {
    assert_ne!(words.len(), 0);
    let mut ret = Self {
      length: length,
      active_words: words.len(),
      words: words,
      cells: (0..length).map(|_| Default::default()).collect(),
    };
    ret.reset_inventories();
    ret
  }

  fn unpick(&mut self, cells: Vec<LineCell>, active_words: usize) {
    //TODO
  }

  fn pick(&mut self, offset: usize, ch: char) {
    // partition the word set down to only those words with this letter in this
    // position.
  }

  fn choose(&mut self, index: usize, ch: char) {
    let active_words = self.active_words;
    let mut begin = 0;
    let mut end = active_words;
    assert_ne!(end, 0);
    loop {
      while begin < end
        && self.words[begin].chars().skip(index).next() == Some(ch.to_ascii_uppercase())
      {
        begin += 1;
      }
      while begin < end
        && self.words[end - 1].chars().skip(index).next() != Some(ch.to_ascii_uppercase())
      {
        end -= 1;
      }
      if begin == end {
        break;
      }
      self.words.swap(begin, end - 1);
    }

    self.active_words = end;
    self.reset_inventories();
  }
}

struct Cell {
  row: usize,
  col: usize,
  down: usize,
  down_offset: usize,
  across: usize,
  across_offset: usize,
  choice: Option<char>,
  char_dist: LetterInventory,
}

struct Choice {}

pub struct Crossword<'a> {
  width: usize,
  height: usize,
  acrosses: Vec<Line<'a>>,
  downs: Vec<Line<'a>>,
  cells: Vec<Cell>,
  choices: Vec<Choice>,
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
    let mut acrosses: Vec<Line> = (0..height)
      .map(|_| line_inits[width - 1].clone().unwrap())
      .collect();
    let mut downs: Vec<Line> = (0..width)
      .map(|_| line_inits[height - 1].clone().unwrap())
      .collect();
    let mut cells: Vec<Cell> = vec![];
    for i in 0..height {
      for j in 0..width {
        let cell_index = cells.len();
        cells.push(Cell {
          row: i,
          col: j,
          down: j,
          down_offset: i,
          across: i,
          across_offset: j,
          choice: None,
          char_dist: LetterInventory::product(
            &downs[j].cells[i].inventory,
            &acrosses[i].cells[j].inventory,
          ),
        });
        downs[j].cells[i].cell_index = cell_index;
        acrosses[i].cells[j].cell_index = cell_index;
      }
    }

    Self {
      width,
      height,
      acrosses,
      downs,
      cells,
      choices: vec![],
    }
  }

  pub fn choose_one(&mut self) -> bool {
    let cell_index = (0..self.cells.len())
      .filter(|&i| self.cells[i].choice.is_none())
      .min_by_key(|&i| self.cells[i].char_dist.letter_set().len())
      .unwrap();
    let inventory = &self.cells[cell_index].char_dist;
    let mut rng = rand::thread_rng();
    let set = inventory.letter_set();
    //dbg!("{:?}", &set);
    assert_ne!(set.len(), 0, "{:?}", &inventory);
    // TODO: WeightedIndex
    let ch = set
      .chars()
      .skip(Uniform::from(0..set.len()).sample(&mut rng))
      .take(1)
      .next()
      .unwrap();
    let cell = &mut self.cells[cell_index];
    cell.choice = Some(ch);
    let (across, across_offset, down, down_offset, row, col) = (
      cell.across,
      cell.across_offset,
      cell.down,
      cell.down_offset,
      cell.row,
      cell.col,
    );
    //dbg!( "A={:?} & B={:?}", &self.downs[down].cells[down_offset].inventory, &self.acrosses[across].cells[across_offset].inventory,);
    self.downs[down].choose(down_offset, ch);
    self.acrosses[across].choose(across_offset, ch);
    let (cells, acrosses, downs) = (&mut self.cells, &self.acrosses, &self.downs);
    for lc in downs[down]
      .cells
      .iter()
      .chain(acrosses[across].cells.iter())
    {
      let c = &mut cells[lc.cell_index];
      let a = &downs[c.down].cells[c.down_offset].inventory;
      let b = &acrosses[c.across].cells[c.across_offset].inventory;
      let prod = LetterInventory::product(a, b);
      assert_ne!(
        prod.letter_set().len(),
        0,
        "{}@{},{} <- {:?} vs {:?}",
        ch,
        row,
        col,
        a,
        b
      );
      c.char_dist = prod;
    }

    self.choices.push(Choice {
      // TODO

    });
    true
  }

  fn undo_one(&mut self) {}

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
      let _ = getch();
      self.choose_one();
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
          addch(' ' as u32);
        }
      }
    }
  }
}
