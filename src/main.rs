#![feature(clamp)]
extern crate ncurses;
extern crate time;
extern crate tui;
extern crate words;

mod crossword;

use crossword::Crossword;
use ncurses::*;
use tui::View;

fn main() {
    initscr();
    clear();
    refresh();
    ncurses::noecho();
    clear();
    Crossword::new(7, 6).ok().unwrap().interact();
}
