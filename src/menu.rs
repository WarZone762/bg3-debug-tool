use hudhook::ImguiRenderLoop;
use imgui::{TableFlags, TextureId, Ui};

use crate::{
    game_definitions::{GameObjectTemplate, GamePtr},
    globals::Globals,
    hooks::vulkan::ImGuiMenu,
    info,
};

pub(crate) struct Menu {
    text: String,
    search_items: Vec<GamePtr<GameObjectTemplate>>,
    texture: TextureId,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

impl Menu {
    pub fn new() -> Self {
        Self { text: String::new(), search_items: Vec::new(), texture: TextureId::new(0) }
    }

    fn search(&mut self) {
        self.search_items.clone_from(&search(&self.text));
    }

    fn render(&mut self, ui: &mut Ui) {
        ui.window("Baldur's Gate 3 Debug Tool")
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .size([200.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                // let min = ui.item_rect_min();
                // ui.get_window_draw_list()
                //     .add_image(self.texture, [min[0] + 50.0, min[1] + 50.0], [
                //         min[0] + 100.0,
                //         min[1] + 100.0,
                //     ])
                //     .build();
                ui.text("Search");
                ui.text(">>");
                ui.same_line();
                if ui.input_text("<<", &mut self.text).enter_returns_true(true).build() {
                    self.search();
                    for e in self.search_items.iter().filter(|x| x.get_type().as_str() == "item") {
                        info!("{} {}", e.name.as_str(), e.id.as_str())
                    }
                }

                if let Some(tbl) = ui.begin_table_with_sizing(
                    "items_tbl",
                    3,
                    TableFlags::SCROLL_Y,
                    [0.0, 100.0],
                    0.0,
                ) {
                    ui.table_next_row();
                    for i in &self.search_items {
                        ui.table_set_column_index(0);

                        ui.text(i.name.as_str());
                        ui.table_next_column();
                        ui.text(i.id.as_str());
                        ui.table_next_column();
                        ui.text(i.get_type().as_str());
                        ui.table_next_column();
                    }
                    tbl.end();
                }
            });
    }
}

impl ImguiRenderLoop for Menu {
    fn initialize<'a>(&'a mut self, ctx: &mut imgui::Context, loader: hudhook::TextureLoader<'a>) {
        ctx.set_ini_filename(None);
        ctx.set_log_filename(None);
        self.texture = loader(&[255; 50 * 50 * 4], 50, 50).unwrap();
    }

    fn render(&mut self, ui: &mut Ui) {
        self.render(ui);
    }
}

impl ImGuiMenu for Menu {
    fn render(&mut self, ui: &mut imgui::Ui) {
        self.render(ui);
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
