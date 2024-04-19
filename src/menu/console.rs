use imgui::Ui;

use crate::wrappers::osiris::FunctionCall;

#[derive(Debug)]
pub(crate) struct Console {
    text: String,
    output: String,
}

impl Console {
    pub fn new() -> Self {
        Self { text: String::new(), output: String::new() }
    }

    pub fn render(&mut self, ui: &Ui) {
        ui.text("Console");
        ui.text(">>");
        ui.same_line();
        if ui.input_text("##input", &mut self.text).enter_returns_true(true).build() {
            self.output.push_str(&format!(">> {}\n", self.text));
            self.run();
        }
        if ui.button("Clear") {
            self.output.clear();
        }
        ui.input_text_multiline("##output", &mut self.output, [-1.0, -1.0]).read_only(true).build();
    }

    pub fn run(&mut self) {
        let call = syn::parse_str::<FunctionCall>(&self.text);
        match call {
            Ok(x) => match x.call() {
                Ok(Some(x)) => self.output.push_str(&x.to_string()),
                Ok(None) => (),
                Err(x) => self.output.push_str(&x.to_string()),
            },
            Err(x) => self.output.push_str(&x.to_string()),
        }
        self.output.push('\n');
    }
}
