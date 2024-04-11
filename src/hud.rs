use hudhook::ImguiRenderLoop;
use imgui::Ui;

use crate::{
    game_definitions::{FixedString, GameObjectTemplate, GamePtr, LSStringView, STDString},
    globals::Globals,
    info,
};

pub(crate) struct Hud {
    text: String,
    search_items: Vec<GamePtr<GameObjectTemplate>>,
}

unsafe impl Send for Hud {}
unsafe impl Sync for Hud {}

impl Hud {
    pub fn new() -> Self {
        Self { text: String::new(), search_items: Vec::new() }
    }

    fn search(&mut self) {
        self.search_items.clone_from(&search(&self.text));
    }
}

impl ImguiRenderLoop for Hud {
    fn render(&mut self, ui: &mut Ui) {
        ui.window("Baldur's Gate 3 Debug Tool")
            .position([0f32, 0f32], imgui::Condition::FirstUseEver)
            .size([320f32, 200f32], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Console");
                ui.text(">>");
                ui.same_line();
                if ui.input_text("<<", &mut self.text).enter_returns_true(true).build() {
                    self.search();
                    for e in self.search_items.iter().filter(|x| x.get_type().as_str() == "item") {
                        info!("{} {}", e.name.as_str(), e.id.as_str())
                    }
                }
            });
    }
}

fn search(text: &str) -> Vec<GamePtr<GameObjectTemplate>> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank
        .templates
        .iter()
        .filter(|x| !x.is_null() && x.value.name.as_str().contains(text))
        .map(|x| x.value)
        .collect()
}
