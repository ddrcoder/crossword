use ncurses::*;
use std::cell::RefCell;
use std::str::FromStr;
use std::rc::Rc;

#[derive(Clone,Copy)]
enum Direction {
    Forward,
    Backward,
}

pub trait View {
    fn interact(&mut self) {
        self.cycle_focus(Direction::Forward);
        loop {
            self.render(0, 0);
            self.cursor(0, 0);
            match getch() {
                9 => {
                    self.cycle_focus(Direction::Forward);
                }
                27 => {
                    match getch() {
                        91 => {
                            match getch() {
                                65 => {
                                    self.cycle_focus(Direction::Backward) ||
                                    self.cycle_focus(Direction::Forward);
                                }
                                66 => {
                                    self.cycle_focus(Direction::Forward) ||
                                    self.cycle_focus(Direction::Backward);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                key => {
                    print!("<{}>", key);
                    self.handle(key);
                }
            }
        }
    }

    fn cycle_focus(&mut self, Direction) -> bool {
        false
    }
    fn focus(&self) -> bool {
        false
    }
    fn handle(&mut self, i32) {}
    fn render(&self, i32, i32) {}
    fn cursor(&self, i32, i32) {}
}

pub struct Label {
    text: String,
}

impl Label {
    pub fn new(text: &str) -> Label {
        Label { text: String::from_str(text).unwrap() }
    }
}

impl View for Label {
    fn render(&self, x: i32, y: i32) {
        mv(y, x);
        printw(&self.text);
    }
}

pub struct TextBox {
    content: Rc<RefCell<String>>,
}

impl TextBox {
    pub fn new(content: Rc<RefCell<String>>) -> TextBox {
        TextBox { content: content }
    }
}

impl View for TextBox {
    fn focus(&self) -> bool {
        true
    }
    fn handle(&mut self, code: i32) {
        match code {
            0x7f => {
                (*self.content).borrow_mut().pop();
            }
            ch => {
                (*self.content).borrow_mut().push(ch as u8 as char);
            }
        }
    }

    fn render(&self, x: i32, y: i32) {
        mv(y, x);
        printw(&*self.content.borrow());
        printw(&"   \x08\x08\x08");
    }
    fn cursor(&self, x: i32, y: i32) {
        mv(y, x + (self.content.borrow().len() as i32));
    }
}

pub struct ListView {
    focus: usize,
    views: Vec<(Box<View>, (i32, i32))>,
}

impl ListView {
    pub fn new() -> ListView {
        ListView {
            focus: 0, // 0 is off the beginning, len + 1 is off the end
            views: Vec::new(),
        }
    }
    pub fn add<V: View + 'static>(mut self, view: V, x: i32, y: i32) -> Self {
        self.views.push((Box::new(view), (x, y)));
        self
    }
}

impl View for ListView {
    fn handle(&mut self, key: i32) {
        assert!(self.focus >= 1 && self.focus - 1 < self.views.len());
        self.views[self.focus - 1].0.handle(key)
    }
    fn cycle_focus(&mut self, dir: Direction) -> bool {
        if self.focus >= 1 && self.focus - 1 < self.views.len() {
            if self.views[self.focus].0.cycle_focus(dir) {
                return true;
            }
        }
        match dir {
            Direction::Forward => {
                while self.focus < self.views.len() {
                    self.focus += 1;
                    if self.views[self.focus - 1].0.focus() {
                        return true;
                    }
                }
                self.focus = self.views.len() + 1;
                return false;
            }
            Direction::Backward => {
                while self.focus > 1 {
                    self.focus -= 1;
                    if self.views[self.focus - 1].0.focus() {
                        return true;
                    }
                }
                self.focus = 0;
                return false;
            }
        }
    }

    fn render(&self, x: i32, y: i32) {
        for &(ref child, (u, v)) in &self.views {
            child.render(x + u, y + v)
        }
    }

    fn cursor(&self, x: i32, y: i32) {
        assert!(self.focus >= 1 && self.focus - 1 < self.views.len());
        let &(ref child, (u, v)) = &self.views[self.focus - 1];
        child.cursor(x + u, y + v);
    }
}
