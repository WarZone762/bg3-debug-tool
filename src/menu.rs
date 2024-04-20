use hudhook::ImguiRenderLoop;
use imgui::{sys::igGetMainViewport, Ui};

use crate::{globals::Globals, hooks::vulkan::ImGuiMenu};

mod console;
mod search;

// TODO:
// - [ ] add console history, clearing console on enter
// - [x] add ability to copy item fields
// - [ ] add ability to add/remove fields in object data
// - [ ] fix selectable in table not covering all row
// - [ ] add resizing table, ability add/remove columns
// - [ ] add Osiris function search
// - [ ] finish other tabs

pub(crate) struct Menu {
    search: search::Search,
    console: console::Console,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

impl Menu {
    pub fn new() -> Self {
        Self { search: search::Search::new(), console: console::Console::new() }
    }

    fn render(&mut self, ui: &Ui) {
        let viewport_pos = unsafe { (*igGetMainViewport()).WorkPos };
        let viewport_size = unsafe { (*igGetMainViewport()).WorkSize };

        ui.window("Baldur's Gate 3 Debug Tool")
            .position(
                [viewport_pos.x + viewport_size.x * 0.75 - 10.0, 10.0],
                imgui::Condition::FirstUseEver,
            )
            .size([viewport_size.x / 4.0, viewport_size.y / 4.0], imgui::Condition::FirstUseEver)
            .collapsed(true, imgui::Condition::FirstUseEver)
            .build(|| {
                if let Some(tab_bar) = ui.tab_bar("tab-bar") {
                    if let Some(item) = ui.tab_item("Object Explorer") {
                        self.search.render(ui);
                        item.end()
                    }
                    if let Some(item) = ui.tab_item("Spells") {
                        item.end()
                    }
                    if let Some(item) = ui.tab_item("Passives & Conditions") {
                        item.end()
                    }
                    if let Some(item) = ui.tab_item("Console") {
                        self.console.render(ui);
                        item.end()
                    }
                    if let Some(item) = ui.tab_item("Info") {
                        item.end()
                    }
                    if let Some(item) = ui.tab_item("Log") {
                        ui.input_text_multiline("Log", &mut Globals::log(), [-1.0, -1.0])
                            .read_only(true)
                            .build();
                        item.end()
                    }
                    tab_bar.end();
                }
            });
    }
}

impl ImguiRenderLoop for Menu {
    fn initialize<'a>(&'a mut self, ctx: &mut imgui::Context, _loader: hudhook::TextureLoader<'a>) {
        ctx.set_ini_filename(None);
        ctx.set_log_filename(None);
    }

    fn render(&mut self, ui: &mut Ui) {
        self.render(ui);
    }
}

impl ImGuiMenu<ash::Device> for Menu {
    fn initialize(&mut self, ctx: &mut imgui::Context, _params: &mut ash::Device) {
        ctx.set_ini_filename(None);
        ctx.set_log_filename(None);
        let io = ctx.io_mut();

        io.config_flags |= imgui::ConfigFlags::NAV_ENABLE_KEYBOARD;
        io.config_flags |= imgui::ConfigFlags::NAV_ENABLE_GAMEPAD;
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        self.render(ui);
    }
}
