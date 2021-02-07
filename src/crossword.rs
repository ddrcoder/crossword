extern crate priority_queue;
extern crate rand;

use crate::skip_iter::{and, diff, leaf, short_leaf};
use ncurses::*;
use priority_queue::PriorityQueue;
use rand::{rngs::ThreadRng, Rng};
use std::collections::hash_set::HashSet;
use std::io::Result;
use tui::View;
use words::dictionary::{english_scrabble_dict, Dictionary};
use words::{LetterInventory, LetterSet};

#[derive(Clone, Debug, Default)]
struct Line {
  length: usize,
  words: Vec<u32>,
  cells: Vec<u32>,
  inventories: Vec<LetterInventory>,
  claimed: Option<u32>,
}

enum ConstrainResult {
  Ok,
  Failed,
  Unique(u32),
}

impl Line {
  fn reset_inventories(&mut self, dictionary: &Dictionary) {
    for inv in &mut self.inventories {
      *inv = LetterInventory::new();
    }
    let (words, inventories) = (self.words.iter().cloned(), &mut self.inventories);
    dictionary.visit_indices(words, |_, str| {
      for (i, ch) in str.chars().enumerate() {
        inventories[i].add(ch);
      }
    });
  }

  fn new(length: usize, words: &[u32]) -> Self {
    Self {
      length: length,
      words: words.iter().cloned().collect(),
      cells: (0..length).map(|_| Default::default()).collect(),
      inventories: (0..length).map(|_| Default::default()).collect(),
      claimed: None,
    }
  }

  fn constrain(
    &mut self,
    filter_set: &[u32],
    claimed_set: &[u32],
    dictionary: &Dictionary,
  ) -> ConstrainResult {
    let count_before = self.words.len();
    let mut claimed_word = [0];
    let claimed_word = if let Some(claimed) = self.claimed {
      claimed_word[0] = claimed;
      &claimed_word[0..1]
    } else {
      &claimed_word[0..0]
    };
    self.words = if self.words.len() < 256 {
      diff(
        and(short_leaf(&self.words[..]), leaf(filter_set)),
        diff(short_leaf(claimed_set), short_leaf(claimed_word)),
      )
      .collect()
    } else {
      diff(
        and(leaf(&self.words[..]), leaf(filter_set)),
        diff(short_leaf(claimed_set), short_leaf(claimed_word)),
      )
      .collect()
    };

    let count_after = self.words.len();
    self.reset_inventories(dictionary);
    match (count_before, count_after) {
      (_, 0) => ConstrainResult::Failed,
      (1, 1) => ConstrainResult::Ok,
      (_, 1) => {
        self.claimed = Some(self.words[0]);
        ConstrainResult::Unique(self.words[0])
      }
      _ => ConstrainResult::Ok,
    }
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
pub struct Choice {
  cell_index: usize,
  line_undo: Vec<(u32, Line, Option<u32>)>,
}

#[derive(Default)]
pub struct WordIndices {
  length_words: Vec<Vec<u32>>,
  //x[length][index][char][]
  length_index_char_words: Vec<Vec<Vec<Vec<u32>>>>,
  length_claimed_words: Vec<Vec<u32>>,
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
    ret.length_claimed_words = vec![vec![]; ret.length_words.len() + 1];
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
  fn claim(&mut self, length: usize, w: u32) {
    let v = &mut self.length_claimed_words[length];
    match v[..].binary_search(&w) {
      Ok(_) => panic!("Already added {}!", w),

      Err(index) => {
        v.insert(index, w);
      }
    }
  }
  fn unclaim(&mut self, length: usize, w: u32) {
    let v = &mut self.length_claimed_words[length];
    match v[..].binary_search(&w) {
      Ok(index) => {
        v.remove(index);
      }
      Err(_) => panic!("{} no claimed!", w),
    }
  }

  fn with_length_claimed(&self, length: usize) -> Option<&[u32]> {
    Some(&(self.length_claimed_words.get(length)?)[..])
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
  tight_cells: PriorityQueue<u32, usize>,
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
      tight_cells: Default::default(),
    };
    for i in 0..ret.cells.len() {
      ret.update_cell(i);
    }
    Ok(ret)
  }

  pub fn update_cell(&mut self, index: usize) {
    let c = &mut self.cells[index];
    if c.choice.is_some() {
      return;
    }
    let a = &self.lines[c.lines[0].0 as usize].inventories[c.lines[0].1 as usize];
    let b = &self.lines[c.lines[1].0 as usize].inventories[c.lines[1].1 as usize];
    let prod = LetterInventory::product(a, b);
    self
      .tight_cells
      //.push(index as u32, prod.tletter_set().len())
      .push(index as u32, !prod.total() as usize);
    c.char_dist = prod;
  }

  fn get_next_choices(&self, rng: &mut ThreadRng) -> Choices {
    if let Some((cell_index, n)) = self.tight_cells.peek() {
      let cell_index = *cell_index as usize;
      let inventory = &self.cells[cell_index].char_dist;
      let set = inventory.letter_set();
      //println!("cell {} has {}", cell_index, !n);
      match set.len() {
        0 => Choices::Failure,
        1 => Choices::Single(cell_index, inventory.letter_set().chars().next().unwrap()),
        _ => {
          let mut to_draw: Vec<_> = inventory
            .entries()
            .map(|(ch, n)| (ch, n * 1000 + (ch as u32)))
            //.map(|(ch, n)| (ch, rng.gen::<f32>().ln() / -(n as f32)))
            .collect();
          to_draw.sort_unstable_by(|(_, t1), (_, t2)| t2.cmp(t1));
          Choices::Many(cell_index, to_draw.into_iter().map(|(ch, _)| ch).collect())
        }
      }
    } else {
      Choices::Success
    }
  }

  pub fn choose(&mut self, cell_index: usize, ch: char) -> Option<Choice> {
    let cell = &mut self.cells[cell_index];
    if cell.choice != None || cell.char_dist.count(ch) == 0 {
      return None;
    }
    self.tight_cells.remove(&(cell_index as u32));
    cell.choice = Some(ch);
    let lines = cell.lines;
    let mut choice = Choice {
      cell_index,
      ..Default::default()
    };
    for (slot, (index, offset)) in lines[..].iter().cloned().enumerate() {
      let line = &mut self.lines[index as usize];

      let save_line = line.clone();
      match line.constrain(
        self
          .word_indices
          .with_length_char_at(line.length, ch, offset as usize)
          .unwrap(),
        self.word_indices.with_length_claimed(line.length).unwrap(),
        &self.dictionary,
      ) {
        ConstrainResult::Unique(claimed) => {
          choice.line_undo.push((index, save_line, Some(claimed)));
          self.word_indices.claim(line.length, claimed);
        }
        ConstrainResult::Ok => {
          choice.line_undo.push((index, save_line, None));
        }
        ConstrainResult::Failed => {
          self.undo(choice);
          return None;
        }
      }
      let cells = self.lines[index as usize].cells.clone();
      for lc in cells {
        self.update_cell(lc as usize);
      }
    }

    Some(choice)
  }

  fn undo(&mut self, choice: Choice) {
    let cell_index = choice.cell_index;
    //println!("unchoosing cell {}", cell_index);
    let cell = &mut self.cells[cell_index];
    mv(cell.row as i32, cell.col as i32);
    addch('_' as u32);
    cell.choice = None;
    for (index, saved_line, claimed) in choice.line_undo {
      let line = &mut self.lines[index as usize];
      *line = saved_line;
      let cells = line.cells.clone();
      let length = line.length;
      for lc in cells {
        self.update_cell(lc as usize);
      }
      if let Some(claimed) = claimed {
        self.word_indices.unclaim(length, claimed);
      }
    }
  }

  fn undo_one(&mut self) -> bool {
    if let Some(choice) = self.choices.pop() {
      self.undo(choice);
      true
    } else {
      false
    }
  }

  pub fn solve(&mut self, rng: &mut ThreadRng) -> std::result::Result<usize, usize> {
    let start = std::time::Instant::now();
    let mut count = 0;
    if self.rec(&mut count, rng) {
      let end = std::time::Instant::now();
      mv(0, 10);
      addstr(&format!("{}s ", (end - start).as_secs_f32()));
      Ok(count)
    } else {
      Err(count)
    }
  }

  fn rec(&mut self, c: &mut usize, rng: &mut ThreadRng) -> bool {
    match self.get_next_choices(rng) {
      Choices::Failure => {
        *c += 1;
        if *c % 0x1000 == 0 {
          self.render(0, 0);
          refresh();
        }
        false
      }
      Choices::Success => true,
      Choices::Single(cell_index, ch) => {
        if let Some(choice) = self.choose(cell_index, ch) {
          self.choices.push(choice);
          if self.rec(c, rng) {
            return true;
          }
          let choice = self.choices.pop().unwrap();
          self.undo(choice);
        }
        false
      }
      Choices::Many(cell_index, chars) => {
        for ch in chars {
          if let Some(choice) = self.choose(cell_index, ch) {
            self.choices.push(choice);
            if self.rec(c, rng) {
              return true;
            }
            let choice = self.choices.pop().unwrap();
            self.undo(choice);
          }
        }
        false
      }
    }
  }

  fn prefilter(&mut self) -> f64 {
    let mut lines_to_update: HashSet<_> = (0..self.lines.len()).collect();
    let mut reduction = 1.;
    while !lines_to_update.is_empty() {
      let mut touched_lines = HashSet::new();
      for l in lines_to_update {
        let line = &self.lines[l];
        let mut new_words = vec![];
        new_words.reserve(line.words.len());
        self
          .dictionary
          .visit_indices(line.words.iter().cloned(), |w, str| {
            let mut bad_letters = str.chars().enumerate().filter(|(i, ch)| {
              let cell_index = line.cells[*i] as usize;
              let cell: &Cell = &self.cells[cell_index];
              let other_inventories = cell.lines[..].iter().filter_map(|(l2, i2)| {
                if *l2 as usize == l {
                  None
                } else {
                  Some(&self.lines[*l2 as usize].inventories[*i2 as usize])
                }
              });

              let mut conflicts = other_inventories.filter(|inv| inv.count(*ch) == 0);
              conflicts.next().is_some()
            });
            if let None = bad_letters.next() {
              new_words.push(w);
            }
          });
        if new_words.len() != line.words.len() {
          reduction *= line.words.len() as f64 / new_words.len() as f64;
          let (line, dictionary) = (&mut self.lines[l], &self.dictionary);
          line.words = new_words;
          line.reset_inventories(&dictionary);
          touched_lines.insert(l);
        }
      }
      lines_to_update = touched_lines;
    }
    reduction
  }

  fn naive_solution_count(&self) -> f64 {
    self
      .lines
      .iter()
      .map(|line| line.words.len() as f64)
      .product()
  }
}

fn clamp(lo: i32, mid: i32, hi: i32) -> i32 {
  if mid < lo {
    lo
  } else if mid > hi {
    hi
  } else {
    mid
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
    let mut msg_line = 0;
    loop {
      x = clamp(0, x, self.width as i32 - 1);
      y = clamp(0, y, self.height as i32 - 1);
      self.render(0, 0);
      if self.choices.len() == self.height * self.width {
        self.cursor(self.width as i32, self.height as i32);
      } else {
        self.cursor(x, y);
      }
      let input = getch() as u8;
      let message: Option<String> = match input as u8 {
        0x9 => {
          downward = !downward;
          None
        }
        0x5c => Some(format!("{}x reduction", self.prefilter())),
        0x7f => {
          // backspace
          if let Some(choice) = self.choices.last() {
            let cell = &self.cells[choice.cell_index];
            x = cell.col as i32;
            y = cell.row as i32;
            self.undo_one();
            Some(format!("undoing one"))
          } else {
            Some(format!("nothing to undo!"))
          }
        }
        //ch @ (0x41..=0x69) |
        ch @ (0x61..=0x79) => {
          let cell_index = (0..self.cells.len())
            .filter(|ci| {
              let cell = &self.cells[*ci];
              (cell.col as i32, cell.row as i32) == (x, y)
            })
            .next()
            .unwrap();
          let ch = (ch as char).to_ascii_uppercase();
          let ret = if let Some(existing) = self.cells[cell_index].choice {
            if existing == ch {
              None
            } else {
              Some(format!("Already filled (backspace it)"))
            }
          } else if let Some(choice) = self.choose(cell_index, ch) {
            self.choices.push(choice);
            Some(format!("{} added!", ch))
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
        0x20 => Some(match self.solve(&mut rng) {
          Ok(steps) => format!(
            "Solved in {} steps with {} choices",
            steps,
            self.choices.len()
          ),
          Err(steps) => format!("Unsolvable, tried {} steps", steps),
        }),
        0x60 => {
          while self.undo_one() {}
          None
        }
        //escape
        0x1b => {
          let input2 = getch() as u8;
          match input2 {
            0x5b => {
              let input3 = getch() as u8;
              match input3 {
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
                _ => Some(format!("{:x}, {:x}", input2, input3)),
              }
            }
            _ => Some(format!("{:x}", input2)),
          }
        }
        0x2c => Some(format!("Narrower")),
        0x2e => Some(format!("Wider")),
        0x2d => Some(format!("Shorter")),
        0x3d => Some(format!("Taller")),
        c => Some(format!("Unrecognized")),
      };
      msg_line += 1;
      if msg_line == 10 {
        msg_line = 0;
      }
      self.cursor(0, self.height as i32 + 2 + msg_line);
      if let Some(s) = message {
        addstr(&format!("{:x}: {}", input, s));
      } else {
        addstr(&format!("{:x}: {} choices", input, self.choices.len()));
      }
      addstr("                         ");
      refresh();
    }
  }

  fn render(&self, x: i32, y: i32) {
    let mut height = 0;
    getmaxyx(stdscr(), &mut height, &mut 0);
    mv(1, 10);
    addstr(&format!(
      "{} solutions seem to remain.                                                           ",
      self.naive_solution_count()
    ));
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
