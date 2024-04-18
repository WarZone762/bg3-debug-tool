use std::ffi::CString;

use hudhook::ImguiRenderLoop;
use imgui::{sys::igGetMainViewport, TableFlags, Ui};

use crate::{
    err,
    game_definitions::{EoCGameObjectTemplate, GameObjectTemplate, ItemTemplate, Template},
    globals::Globals,
    hooks::vulkan::ImGuiMenu,
    wrappers::osiris,
};

pub(crate) struct Menu {
    text: String,
    search_items: Vec<SearchItem>,
    selected_item: Option<usize>,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

impl Menu {
    pub fn new() -> Self {
        Self { text: String::new(), search_items: Vec::new(), selected_item: None }
    }

    fn search(&mut self) {
        self.search_items.clear();
        self.search_items.extend(templates().filter(|x| {
            let text = self.text.to_lowercase();
            match x {
                SearchItem::Item(x) => {
                    x.name.to_lowercase().contains(&text)
                        || x.display_name.as_ref().is_some_and(|x| x.to_lowercase().contains(&text))
                }
                SearchItem::Other(x) => {
                    x.name.to_lowercase().contains(&text)
                        || x.display_name.as_ref().is_some_and(|x| x.to_lowercase().contains(&text))
                }
            }
        }));
    }

    fn render_item_search(&mut self, ui: &Ui) {
        let size = ui.window_size();
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
        if ui.input_text(" ", &mut self.text).enter_returns_true(true).build() {
            self.search();
        }

        if let Some(tbl) = ui.begin_table_with_sizing(
            "items_tbl",
            2,
            TableFlags::SCROLL_Y,
            [size[0] * 0.5, size[1] - 100.0],
            0.0,
        ) {
            ui.table_next_row();
            for (i, item) in self.search_items.iter().enumerate() {
                ui.table_set_column_index(0);

                if ui
                    .selectable_config(item.name())
                    .span_all_columns(true)
                    .selected(self.selected_item.is_some_and(|x| x == i))
                    .build()
                {
                    self.selected_item = Some(i);
                }

                ui.table_next_column();
                ui.text(item.display_name().unwrap_or("[NO NAME]"));
                ui.table_next_column();
            }
            tbl.end();
        }

        ui.same_line();
        ui.child_window("Objec Data").build(|| {
            if let Some(selected_item) = self.selected_item
                && let Some(item) = self.search_items.get(selected_item)
            {
                item.render(ui);
            }
        });
    }

    fn render(&mut self, ui: &Ui) {
        let viewport_pos = unsafe { (*igGetMainViewport()).WorkPos };
        let viewport_size = unsafe { (*igGetMainViewport()).WorkSize };

        ui.window("Baldur's Gate 3 Debug Tool")
            .position(
                [viewport_pos.x + viewport_size.x * 0.75, 0.0],
                imgui::Condition::FirstUseEver,
            )
            .size([viewport_size.x / 4.0, viewport_size.y / 4.0], imgui::Condition::FirstUseEver)
            .collapsed(true, imgui::Condition::FirstUseEver)
            .build(|| {
                if let Some(tab_bar) = ui.tab_bar("tab_bar") {
                    if let Some(item) = ui.tab_item("Object Explorer") {
                        self.render_item_search(ui);
                        item.end()
                    }
                    if let Some(item) = ui.tab_item("Console") {
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
        // self.texture = loader(&[255; 50 * 50 * 4], 50, 50).unwrap();
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

#[derive(Debug, Clone)]
pub(crate) enum SearchItem {
    // CombinedLight
    // LevelTemplate
    // Schematic
    // Spline
    // TileConstruction
    // character
    // constellation
    // constellationHelper
    // decal
    // fogVolume
    // item
    // light
    // lightProbe
    // prefab
    // projectile
    // scenery
    // surface
    // terrain
    // trigger
    Item(Item),
    Other(Other),
}

impl From<Template<'_>> for SearchItem {
    fn from(value: Template) -> Self {
        match value {
            Template::GameObject(x) => Self::Other(x.into()),
            Template::EoCGameObject(x) => Self::Other(x.into()),
            Template::Scenery(x) => Self::Other((&x.base).into()),
            Template::Item(x) => Self::Item(x.into()),
        }
    }
}

impl SearchItem {
    pub fn render(&self, ui: &Ui) {
        match self {
            SearchItem::Item(x) => x.render(ui),
            SearchItem::Other(x) => x.render(ui),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            SearchItem::Item(x) => &x.name,
            SearchItem::Other(x) => &x.name,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            SearchItem::Item(x) => &x.id,
            SearchItem::Other(x) => &x.id,
        }
    }

    pub fn display_name(&self) -> Option<&str> {
        match self {
            SearchItem::Item(x) => x.display_name.as_deref(),
            SearchItem::Other(x) => x.display_name.as_deref(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Item {
    name: String,
    id: String,
    display_name: Option<String>,
    desc: Option<String>,
}

impl From<&ItemTemplate> for Item {
    fn from(value: &ItemTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.try_into().unwrap();
        let display_name = (*value.display_name).try_into().ok();
        let desc = (*value.description).try_into().ok();

        Self { name, id, display_name, desc }
    }
}

impl Item {
    pub fn render(&self, ui: &Ui) {
        if let Some(tbl) = ui.begin_table("obj_data_tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);
            if let Some(display_name) = &self.display_name {
                ui.text("Display Name");
                ui.table_next_column();
                ui.text_wrapped(display_name.as_str());
                ui.table_next_column();
            }

            if let Some(desc) = &self.desc {
                ui.text("Description");
                ui.table_next_column();
                ui.text_wrapped(desc.as_str());
                ui.table_next_column();
            }

            ui.text("GUID");
            ui.table_next_column();
            ui.text_wrapped(&self.id);
            ui.table_next_column();

            ui.text("Name");
            ui.table_next_column();
            ui.text_wrapped(&self.name);
            if ui.button("Give") {
                if let Err(err) = give_item(&self.id) {
                    err!("failed to give item {err}");
                };
            }
            tbl.end()
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Other {
    name: String,
    id: String,
    display_name: Option<String>,
}

impl From<&EoCGameObjectTemplate> for Other {
    fn from(value: &EoCGameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.try_into().unwrap();
        let display_name = (*value.display_name).try_into().ok();

        Self { name, id, display_name }
    }
}

impl From<&GameObjectTemplate> for Other {
    fn from(value: &GameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.try_into().unwrap();

        Self { name, id, display_name: None }
    }
}

impl Other {
    pub fn render(&self, ui: &Ui) {
        if let Some(tbl) = ui.begin_table("obj_data_tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);
            if let Some(display_name) = &self.display_name {
                ui.text("Display Name");
                ui.table_next_column();
                ui.text_wrapped(display_name.as_str());
                ui.table_next_column();
            }

            ui.text("GUID");
            ui.table_next_column();
            ui.text_wrapped(&self.id);
            ui.table_next_column();

            ui.text("Name");
            ui.table_next_column();
            ui.text_wrapped(&self.name);
            if ui.button("Give") {
                if let Err(err) = give_item(&self.id) {
                    err!("failed to give item {err}");
                };
            }
            tbl.end()
        }
    }
}

fn templates() -> impl Iterator<Item = SearchItem> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| SearchItem::from(Template::from(x.value.as_ref())))
}

fn give_item(uuid: &str) -> anyhow::Result<()> {
    osiris::OsiCall {
        ident: "TemplateAddTo".into(),
        args: vec![
            osiris::OsiArg::GuidString(CString::new(uuid).unwrap()),
            get_host_character()?,
            osiris::OsiArg::I32(1),
            osiris::OsiArg::I32(0),
        ],
    }
    .call()
}

fn get_host_character() -> anyhow::Result<osiris::OsiArg> {
    osiris::OsiCall { ident: "GetHostCharacter".into(), args: Vec::new() }.query()
}
