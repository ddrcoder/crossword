extern crate ncurses;
extern crate time;
extern crate tui;
extern crate words;

mod crossword;
mod skip_iter;

use crossword::Crossword;
use ncurses::*;
use tui::View;

fn main() {
    initscr();
    clear();
    refresh();
    ncurses::noecho();
    clear();
    Crossword::new(6, 7).ok().unwrap().interact();
}
