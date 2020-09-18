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
    let mut crossword = Crossword::new(6, 5).ok().unwrap();
    //while crossword.choose_one() {}
    initscr();
    clear();
    refresh();
    ncurses::noecho();
    clear();
    let mut root = crossword;
    root.interact();
}
