use std::ops::DerefMut;

use imgui::{sys::igGetMainViewport, FontConfig, FontGlyphRanges, FontSource, Ui};

use crate::globals::Globals;

pub(crate) mod backend;
mod console;
pub(crate) mod search;

// TODO:
// - [x] add Osiris function search
// - [x] add Osiris function search type options
// - [x] add ability to copy item fields
// - [x] add ability to launch either Vulkan or DX11
// - [x] add console history, clearing console on enter
// - [x] add hotkey to toggle the menu
// - [x] add resizing/adding/removing columns
// - [x] add search total
// - [x] add table header
// - [x] fix selectable in table not covering the entire row's height
// - [x] add DX11 hooks
// - [x] ~~skip loading the Script Extender(DWrite.dll)~~ (works with SE
//   somehow)
// - [x] figure out Osiris value type names
// - [x] add more fields to objects
// - [-] add ability to remove items, spells etc. from the character
// - [-] add ability to export game data
// - [-] finish other categories
//   - [x] Osiris functions
//   - [x] spells
//   - [x] statuses
//   - [x] passives
//   - [ ] reactions
//   - [ ] other template types and ability to spawn them, entities etc.
// - [ ] finish info tab (components, stats, position)
// - [ ] add regex search
// - [ ] replace Win32 backend with SDL2
// - [ ] replace imgui with egui
// - [ ] ***add icons***

pub(crate) struct Menu {
    opened: bool,
    tip_opened: bool,
    search: search::Search,
    console: console::Console,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

impl Menu {
    pub fn new() -> Self {
        Self {
            opened: true,
            tip_opened: true,
            search: search::Search::default(),
            console: console::Console::default(),
        }
    }

    pub fn init(ctx: &mut imgui::Context) {
        ctx.set_ini_filename(None);
        ctx.set_log_filename(None);
        let io = ctx.io_mut();

        io.config_flags |= imgui::ConfigFlags::NAV_ENABLE_KEYBOARD;
        io.config_flags |= imgui::ConfigFlags::NAV_ENABLE_GAMEPAD;

        ctx.fonts().add_font(&[FontSource::TtfData {
            data: include_bytes!("../assets/font/FiraMonoNerdFontPropo-Regular.otf"),
            size_pixels: 16.0,
            config: Some(FontConfig {
                glyph_ranges: FontGlyphRanges::from_slice(&[1, 0x10FFFF, 0]),
                ..Default::default()
            }),
        }]);
    }

    fn render(&mut self, ui: &Ui) {
        let viewport_pos = unsafe { (*igGetMainViewport()).WorkPos };
        let viewport_size = unsafe { (*igGetMainViewport()).WorkSize };

        if ui.is_key_pressed(imgui::Key::F11) {
            self.opened = !self.opened;
        }

        if !self.opened {
            if ui.is_key_pressed(imgui::Key::F9) {
                self.tip_opened = !self.tip_opened;
            }
            if self.tip_opened {
                ui.window("##open-menu-tip")
                    .title_bar(false)
                    .draw_background(false)
                    .movable(false)
                    .position([0.0, 25.0], imgui::Condition::Always)
                    .build(|| {
                        ui.text("Press F11 to open the Debug Menu, F9 to hide this text");
                    });
            }
            return;
        }

        ui.window("Baldur's Gate 3 Debug Tool")
            .position(
                [viewport_pos.x + viewport_size.x * 0.75 - 10.0, 10.0],
                imgui::Condition::FirstUseEver,
            )
            .size([viewport_size.x / 4.0, viewport_size.y / 3.0], imgui::Condition::FirstUseEver)
            .opened(&mut self.opened)
            .build(|| {
                if let Some(tab_bar) = ui.tab_bar("tab-bar") {
                    if let Some(item) = ui.tab_item("Game Data Explorer") {
                        self.search.render(ui);
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

impl ImGuiMenu<ash::Device> for Menu {
    fn init(&mut self, ctx: &mut imgui::Context, _params: &mut ash::Device) {
        Self::init(ctx);
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        self.render(ui);
    }
}

impl ImGuiMenu<()> for Menu {
    fn init(&mut self, ctx: &mut imgui::Context, _params: &mut ()) {
        Self::init(ctx);
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        self.render(ui);
    }
}

pub(crate) trait ImGuiMenu<InitParam> {
    fn init(&mut self, _ctx: &mut imgui::Context, _params: &mut InitParam) {}
    fn pre_render(&mut self, _ctx: &mut imgui::Context) {}
    fn render(&mut self, ui: &mut imgui::Ui);
}

impl<M: ImGuiMenu<InitParam> + ?Sized, InitParam> ImGuiMenu<InitParam> for Box<M> {
    fn init(&mut self, ctx: &mut imgui::Context, params: &mut InitParam) {
        Box::deref_mut(self).init(ctx, params);
    }

    fn pre_render(&mut self, ctx: &mut imgui::Context) {
        Box::deref_mut(self).pre_render(ctx);
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        Box::deref_mut(self).render(ui);
    }
}
