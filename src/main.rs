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
    let mut rng = rand::thread_rng();
    //Crossword::new(4, 4).ok().unwrap().solve(&mut rng);
    initscr();
    clear();
    refresh();
    ncurses::noecho();
    clear();
    Crossword::new(6, 6).ok().unwrap().interact();
}
