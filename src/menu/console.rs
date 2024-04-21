use imgui::{HistoryDirection, InputTextCallback, InputTextCallbackHandler, TextCallbackData, Ui};

use crate::wrappers::osiris::FunctionCall;

#[derive(Debug)]
pub(crate) struct Console {
    text: String,
    output: String,
    history: History,
    reclaim_focus: bool,
}

impl Default for Console {
    fn default() -> Self {
        Self {
            text: String::new(),
            output: String::new(),
            history: History::new(100),
            reclaim_focus: true,
        }
    }
}

impl Console {
    pub fn render(&mut self, ui: &Ui) {
        ui.text("Console");
        ui.text(">>");
        ui.same_line();
        if self.reclaim_focus {
            ui.set_keyboard_focus_here();
            self.reclaim_focus = false;
        }
        if ui
            .input_text("##input", &mut self.text)
            .callback(InputTextCallback::HISTORY, &mut self.history)
            .enter_returns_true(true)
            .build()
        {
            self.run();
        }
        ui.set_item_default_focus();
        ui.same_line();
        if ui.button("Run") {
            self.run();
        }
        if ui.button("Clear") {
            self.output.clear();
        }
        ui.input_text_multiline("##output", &mut self.output, [-1.0, -1.0]).read_only(true).build();
    }

    pub fn run(&mut self) {
        self.output.push_str(&format!(">> {}\n", self.text));
        let call = syn::parse_str::<FunctionCall>(&self.text);
        self.history.insert(self.text.clone());
        self.text.clear();
        match call {
            Ok(x) => match x.call() {
                Ok(Some(x)) => self.output.push_str(&x.to_string()),
                Ok(None) => (),
                Err(x) => self.output.push_str(&x.to_string()),
            },
            Err(x) => self.output.push_str(&x.to_string()),
        }
        self.output.push('\n');
        self.reclaim_focus = true;
    }
}

impl InputTextCallbackHandler for &mut History {
    fn on_history(&mut self, dir: HistoryDirection, mut data: TextCallbackData) {
        match dir {
            HistoryDirection::Up => {
                if let Some(string) = self.prev(data.str()) {
                    data.clear();
                    data.push_str(string);
                }
            }
            HistoryDirection::Down => {
                if let Some(string) = self.next() {
                    data.clear();
                    data.push_str(string);
                }
            }
        }
    }
}

#[derive(Debug)]
struct History {
    buf: Vec<Box<str>>,
    pos: usize,
    size: usize,
    last_item: Option<Box<str>>,
}

impl History {
    pub fn new(size: usize) -> Self {
        Self { buf: Vec::new(), pos: 0, size, last_item: None }
    }

    pub fn prev(&mut self, last_item: &str) -> Option<&str> {
        self.pos = (self.size + 1).min(self.pos + 1).min(self.buf.len());
        if self.pos == 0 {
            return None;
        } else if self.pos == 1 {
            self.last_item = Some(Box::from(last_item));
        }
        Some(&self.buf[self.pos - 1])
    }

    pub fn next(&mut self) -> Option<&str> {
        self.pos = self.pos.saturating_sub(1);
        if self.pos == 0 {
            return self.last_item.as_deref();
        }
        Some(&self.buf[self.pos - 1])
    }

    pub fn insert(&mut self, string: String) {
        self.last_item = None;
        self.pos = 0;
        self.buf.insert(0, string.into_boxed_str());
        if self.buf.len() > self.size {
            self.buf.truncate(self.size);
        }
    }
}
