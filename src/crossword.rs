extern crate priority_queue;
extern crate rand;

use ncurses::*;
use rand::{rngs::ThreadRng, Rng};
use std::collections::hash_set::HashSet;
use std::collections::HashMap;
use std::rc::Rc;
use tui::View;
use words::dictionary::{english_scrabble_dict, Dictionary};
use words::LetterSet;

#[derive(Clone, Debug, Default)]
struct Line {
  direction: u8,
  cell_indices: Vec<u32>,
}

impl Line {
  fn length(&self) -> usize {
    self.cell_indices.len()
  }
}

#[derive(Clone, Debug, Default)]
struct LineState {
  // Length for this set is implied by length of position_letters.
  position_letters: Vec<LetterSet>,
  ords: Vec<u8>,
}

impl LineState {
  fn new(length: usize) -> LineState {
    LineState {
      position_letters: vec![Default::default(); length],
      ords: vec![],
    }
  }

  fn length(&self) -> usize {
    self.position_letters.len()
  }

  fn word(&self, w: usize) -> &[u8] {
    let n = self.length();
    let b = w * n;
    let e = b + n;
    &self.ords[b..e]
  }

  fn word_count(&self) -> usize {
    self.ords.len() / self.length()
  }

  fn add(&mut self, w: &str) {
    self.ords.len();
    let new_len = self.ords.len() + self.length();
    self.ords.reserve(self.length());
    let (ords, sets) = (&mut self.ords, &mut self.position_letters);
    for (o, set) in w.chars().map(LetterSet::index).zip(sets.iter_mut()) {
      let o = o.unwrap();
      set.insert_index(o);
      ords.push(o);
    }
    assert_eq!(new_len, self.ords.len());
  }

  fn add_ords(&mut self, w: &[u8]) {
    self.ords.reserve(w.len());
    let (ords, sets) = (&mut self.ords, &mut self.position_letters);
    for (o, set) in w.iter().zip(sets.iter_mut()) {
      let o = *o;
      set.insert_index(o);
      ords.push(o);
    }
  }

  fn with_chosen(&self, ord: u8, pos: u8) -> LineState {
    let mut ret = LineState::new(self.length());
    for i in 0..self.word_count() {
      let word = self.word(i);
      if word[pos as usize] == ord {
        ret.add_ords(word);
      }
    }
    ret
  }
}

#[derive(Clone, Debug, Default)]
struct Cell {
  lines: [(u32, u8); 2],
}

struct Puzzle {
  lines: Vec<Line>,
  cells: Vec<Cell>,
  cell_positions: Vec<(usize, usize)>,
}

impl Puzzle {
  fn new(grid: &Grid) -> Puzzle {
    let cell_count = grid.squares.len();
    let cell_indices = 0..grid.squares.len();
    let cell_positions: Vec<_> = grid.squares.keys().cloned().collect();
    let loc_to_ci: HashMap<_, _> = cell_positions
      .iter()
      .cloned()
      .zip(cell_indices.clone())
      .collect();
    let mut cells: Vec<Cell> = vec![Default::default(); cell_count];
    let mut lines = vec![];
    for ci in cell_indices.clone() {
      let (x, y) = cell_positions[ci];
      if !loc_to_ci.contains_key(&(x - 1, y)) {
        lines.push(Line {
          direction: 0,
          cell_indices: (0..)
            .map(|i| loc_to_ci.get(&(x + i, y)))
            .take_while(|ci| ci.is_some())
            .map(|ci| *ci.unwrap() as u32)
            .collect(),
        });
      }
    }
    for ci in cell_indices.clone() {
      let (x, y) = cell_positions[ci];
      if !loc_to_ci.contains_key(&(x, y - 1)) {
        lines.push(Line {
          direction: 1,
          cell_indices: (0..)
            .map(|i| loc_to_ci.get(&(x, y + i)))
            .take_while(|ci| ci.is_some())
            .map(|ci| *ci.unwrap() as u32)
            .collect(),
        });
      }
    }
    for (li, line) in lines.iter().cloned().enumerate() {
      for (pos, ci) in line.cell_indices.iter().cloned().enumerate() {
        cells[ci as usize].lines[line.direction as usize] = (li as u32, pos as u8);
      }
    }
    Puzzle {
      lines,
      cells,
      cell_positions,
    }
  }
}

enum SolveResult {
  None,
  Incomplete(Vec<(usize, char)>),
  Solution(Vec<(usize, char)>),
}

#[derive(Clone)]
struct Solver<'a> {
  puzzle: &'a Puzzle,
  line_states: Vec<Rc<LineState>>,
}

impl<'a> Solver<'a> {
  fn new(puzzle: &'a Puzzle, dictionary: &Dictionary) -> Solver<'a> {
    let lengths: HashSet<_> = puzzle.lines.iter().map(|l| l.length()).collect();
    let mut line_state_templates: HashMap<usize, LineState> = lengths
      .into_iter()
      .map(|l| (l, LineState::new(l)))
      .collect();
    dictionary.visit_all(|_, s: &str| {
      if let Some(line) = line_state_templates.get_mut(&s.len()) {
        line.add(s);
      }
    });
    let line_state_templates: HashMap<usize, Rc<LineState>> = line_state_templates
      .into_iter()
      .map(|(k, v)| (k, Rc::from(v)))
      .collect();
    Solver {
      puzzle,
      line_states: puzzle
        .lines
        .iter()
        .map(|line| line_state_templates.get(&line.length()).unwrap().clone())
        .collect(),
    }
  }

  fn max_permutations(&self) -> f64 {
    self
      .line_states
      .iter()
      .map(|line| line.word_count() as f64)
      .product()
  }

  fn cell_set(&self, ci: usize) -> LetterSet {
    let cell = &self.puzzle.cells[ci];
    let lis = &cell.lines;
    LetterSet::intersect(
      &self.line_states[lis[0].0 as usize].position_letters[lis[0].1 as usize],
      &self.line_states[lis[1].0 as usize].position_letters[lis[1].1 as usize],
    )
  }

  fn solved_char(&self, ci: usize) -> Option<char> {
    let cell = &self.puzzle.cells[ci];
    let lis = &cell.lines;
    let s1 = &self.line_states[lis[0].0 as usize].position_letters[lis[0].1 as usize];
    let s2 = &self.line_states[lis[1].0 as usize].position_letters[lis[1].1 as usize];
    let s = LetterSet::intersect(&s1, &s2);
    //assert_eq!(s1, s2);
    if s.len() == 1 {
      s.chars().next()
    } else {
      None
    }
  }

  fn commit_char(&mut self, cell_index: usize, ch: char) -> bool {
    self
      .commit_ord(cell_index, LetterSet::index(ch).unwrap())
      .is_some()
  }

  fn commit_ord(&mut self, ci: usize, ord: u8) -> Option<usize> {
    let cell = &self.puzzle.cells[ci];
    let lis = &cell.lines;
    let mut across = &mut self.line_states[lis[0].0 as usize];
    let mut cost = 0;
    if !across.position_letters[lis[0].1 as usize].contains_index(ord) {
      return None;
    }
    cost += across.ords.len();
    *across = Rc::from(across.with_chosen(ord, lis[0].1));

    let mut down = &mut self.line_states[lis[1].0 as usize];
    if !down.position_letters[lis[1].1 as usize].contains_index(ord) {
      return None;
    }
    cost += down.ords.len();
    *down = Rc::from(down.with_chosen(ord, lis[1].1));

    /* TODO: Confirm that the position_letters only changed the selected
     * offset's letterset. If another position changed, the corresponding line
     * for that position must be refreshed to filter out the affected words.
     * This may cascade, even back to the original line.*/
    Some(cost)
  }

  fn solve(self, budget: &mut usize, depth: usize) -> SolveResult {
    if depth < 20 {
      mv(0, 0);
      addstr(&format!(
        "\r{:*<}{}{} Choices                      ",
        depth * 2,
        "",
        self.max_permutations()
      ));
      refresh();
    }
    let mut best_choice = None;
    for ci in 0..self.puzzle.cells.len() {
      let set = self.cell_set(ci);
      let n = set.len();
      if n == 0 {
        return SolveResult::None;
      }
      if n == 1 {
        continue;
      }
      if let Some((smallest_n, _, _)) = best_choice {
        if smallest_n <= n {
          continue;
        }
      }
      best_choice = Some((n, ci, set));
    }
    if let Some((_, ci, set)) = best_choice {
      for o in set.indices() {
        // TODO:Shuffle
        let mut child = self.clone();
        if let Some(cost) = child.commit_ord(ci, o) {
          if let Some(remaining) = budget.checked_sub(cost) {
            *budget = remaining;
          } else {
            return SolveResult::Incomplete(
              (0..self.puzzle.cells.len())
                .filter_map(|ci| self.solved_char(ci).map(|ch| (ci, ch)))
                .collect(),
            );
          }
        } else {
          // Direct constraint always works, but indirect effects could reveal a dead end.
          continue;
        }
        let result = child.solve(budget, depth + 1);
        match &result {
          SolveResult::Solution(_) | SolveResult::Incomplete(_) => {
            return result;
          }
          _ => {}
        }
      }
      SolveResult::None
    } else {
      SolveResult::Solution(
        (0..self.puzzle.cells.len())
          .map(|ci| (ci, self.solved_char(ci).unwrap()))
          .collect(),
      )
    }
  }
}

#[derive(Clone)]
pub enum Square {
  Empty,
  Fixed(char),
  Solved(char),
}

pub struct Grid {
  // Walls are missing squares.
  squares: HashMap<(usize, usize), Square>,
}

impl Grid {
  pub fn new_rectangle(width: usize, height: usize) -> Grid {
    Grid {
      squares: (1..=height)
        .flat_map(|y| (1..=width).map(move |x| ((x, y), Square::Empty)))
        .collect(),
    }
  }

  pub fn new_circle(outer: i64, inner: i64) -> Grid {
    let c = outer + 1;
    Grid {
      squares: (1..=(c * 2))
        .flat_map(|y| (1..=(c * 2)).map(move |x| (x, y)))
        .filter_map(|(x, y)| {
          let dx = x - c;
          let dy = y - c;
          let r = dx * dx + dy * dy;
          if r <= inner * inner || r >= outer * outer + outer {
            None
          } else {
            Some((x as usize, y as usize))
          }
        })
        .map(|p| (p, Square::Empty))
        .collect(),
    }
  }

  pub fn new_diamond(outer: i64, inner: i64) -> Grid {
    let c = outer + 1;
    Grid {
      squares: (1..=(c * 2))
        .flat_map(|y| (1..=(c * 2)).map(move |x| (x, y)))
        .filter_map(|(x, y)| {
          let mut dx = (x - c).abs();
          let mut dy = (y - c).abs();
          if dx == 0 {
            dy += 1;
          }
          if dy == 0 {
            dy += 1;
          }
          let r = dx + dy;
          if r <= inner || r > outer {
            None
          } else {
            Some((x as usize, y as usize))
          }
        })
        .map(|p| (p, Square::Empty))
        .collect(),
    }
  }

  pub fn get_outline(&self) -> HashSet<(usize, usize)> {
    let mut outline = HashSet::new();
    for ((x, y), _) in &self.squares {
      for dx in 0..=2 {
        let x = x + dx - 1;
        for dy in 0..=2 {
          let y = y + dy - 1;
          let loc = (x, y);
          if !self.squares.contains_key(&loc) {
            outline.insert(loc);
          }
        }
      }
    }
    outline
  }

  pub fn solve(&mut self, dictionary: &Dictionary, _rng: &mut ThreadRng) -> bool {
    let puzzle = Puzzle::new(self);
    let mut solver = Solver::new(&puzzle, dictionary);
    for (ci, position) in puzzle.cell_positions.iter().enumerate() {
      if let Square::Fixed(ch) = self.squares[position] {
        if !solver.commit_char(ci, ch) {
          return false;
        }
      }
    }
    for square in self.squares.values_mut() {
      if let Square::Solved(_) = *square {
        *square = Square::Empty;
      }
    }
    let empty = [];
    let result = solver.solve(&mut 40000000000, 0);
    let (ci_chars, ret) = match &result {
      SolveResult::Incomplete(chars) => (&chars[..], false),
      SolveResult::Solution(chars) => (&chars[..], true),
      SolveResult::None => (&empty[..], false),
    };
    for (ci, ch) in ci_chars {
      let pos = puzzle.cell_positions[*ci];
      let square = self.squares.get_mut(&pos).unwrap();
      match square.clone() {
        Square::Fixed(CH) => {
          //assert_eq!(CH, ch);
        }
        Square::Empty | Square::Solved(_) => {
          *square = Square::Solved(*ch);
        }
      }
    }
    ret
  }

  pub fn set_square(&mut self, x: usize, y: usize, square: Square) {
    use std::collections::hash_map::Entry;
    match self.squares.entry((x, y)) {
      Entry::Occupied(mut slot) => {
        slot.insert(square);
      }
      Entry::Vacant(slot) => {
        slot.insert(square);
      }
    }
  }
}

impl View for Grid {
  fn cursor(&self, x: i32, y: i32) {
    mv(y, x);
  }
  fn interact(&mut self) {
    let (mut x, mut y) = (1, 1);
    let mut rng = rand::thread_rng();
    let mut downward = false;
    let mut msg_line = 0;
    let dictionary = english_scrabble_dict().ok().unwrap();
    loop {
      if x < 1 {
        x = 1;
      }
      if y < 2 {
        y = 2;
      }
      self.render(0, 1);
      self.cursor(x, y);
      let input = getch() as u8;
      let u = x as usize;
      let v = y as usize - 1;
      let message: Option<String> = match input as u8 {
        0x9 => {
          // tab
          downward = !downward;
          None
        }
        0xa => {
          // enter
          if self.solve(&dictionary, &mut rng) {
            Some(format!("Solved!"))
          } else {
            Some(format!("Failed!"))
          }
        }
        0x20 => {
          //clear spot
          self.set_square(u, v, Square::Empty);
          None
        }
        0x7f => {
          // backspace
          self.squares.remove(&(u, v));
          None
        }
        //ch @ (0x41..=0x69) |
        ch @ (0x61..=0x80) => {
          let ch = (ch as char).to_ascii_uppercase();
          self.set_square(u, v, Square::Fixed(ch));
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
        c => Some(format!("Unrecognized")),
      };
      self.cursor(0, 0);
      if let Some(s) = message {
        addstr(&format!("0x{:x}: {}", input, s));
      }
      addstr("                                            ");
      refresh();
    }
  }

  fn render(&self, left: i32, top: i32) {
    for ((x, y), square) in &self.squares {
      mv(*y as i32 + top, *x as i32 + left);

      addch(match square {
        Square::Empty => ' ',
        Square::Fixed(ch) => ch.to_ascii_uppercase(),
        Square::Solved(ch) => ch.to_ascii_lowercase(),
      } as u32);
    }
    for (x, y) in self.get_outline() {
      mv(y as i32 + top, x as i32 + left);

      addch('#' as u32);
    }
  }
}
