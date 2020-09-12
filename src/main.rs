extern crate ncurses;
extern crate time;
extern crate words;
extern crate tui;

mod crossword;

use ncurses::*;
use std::rc::Rc;
use std::cell::RefCell;
use tui::*;
use words::dictionary::english_scrabble;
use crossword::Crossword;

fn main() {
    initscr();
    clear();
    printw("Reading...\n");
    let mut store = String::new();
    let words = english_scrabble(&mut store).unwrap();
    refresh();
    ncurses::noecho();
    clear();
    let crossword = Crossword::new(&words);
    root.interact();
}
