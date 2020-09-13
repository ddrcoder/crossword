extern crate ncurses;
extern crate time;
extern crate tui;
extern crate words;

mod crossword;

use crossword::Crossword;
use ncurses::*;
use tui::*;
use words::dictionary::english_scrabble;

fn main() {
    let mut store = String::new();
    let words = english_scrabble(&mut store).unwrap();
    let mut crossword = Crossword::new(6, 6, &words);
    initscr();
    clear();
    refresh();
    ncurses::noecho();
    clear();
    let mut root = crossword;
    root.interact();
}
