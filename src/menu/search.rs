use std::borrow::Cow;

use imgui::{TableFlags, Ui};

use crate::{
    err,
    game_definitions::{EoCGameObjectTemplate, GameObjectTemplate, ItemTemplate, Template},
    globals::Globals,
    osi_fn,
    wrappers::osiris,
};

#[derive(Debug, Clone)]
pub(crate) struct Search {
    cur_category: usize,
    text: String,
    options: Options,
    items: ItemsCategory,
    other: OtherCategory,
}

macro_rules! cur_category {
    ($this:expr, $f:ident($($arg:expr),*)) => {
        match $this.cur_category {
            0 => $this.items.$f($($arg,)*),
            1 => $this.other.$f($($arg,)*),
            _ => $this.items.$f($($arg,)*),
        }
    };
}

impl Search {
    pub fn new() -> Self {
        Self {
            cur_category: 0,
            text: String::new(),
            options: Options::new(),
            items: ItemsCategory::new(),
            other: OtherCategory::new(),
        }
    }

    pub fn render(&mut self, ui: &Ui) {
        if let Some(node) = ui.tree_node("Search Options") {
            ui.checkbox("Case Sensitive", &mut self.options.case_sensitive);
            if let Some(node) = ui.tree_node("Search Fields") {
                ui.checkbox("Search Name", &mut self.options.search_name);
                ui.checkbox("Search GUID", &mut self.options.search_id);
                ui.checkbox("Search Display Name", &mut self.options.search_display_name);
                cur_category!(self, render_options(ui));
                node.pop();
            }
            node.pop();
        }
        ui.combo("Object Category", &mut self.cur_category, &["Items", "Other"], |x| Cow::from(*x));
        ui.separator();
        ui.text("Search");
        if ui.input_text("<<", &mut self.text).enter_returns_true(true).build() {
            cur_category!(self, search(&self.text, &self.options));
        }

        let size = ui.window_size();
        if let Some(tbl) = ui.begin_table_with_sizing(
            "items-tbl",
            cur_category!(self, columns()),
            TableFlags::SCROLL_Y,
            [size[0] * 0.5, -1.0],
            0.0,
        ) {
            ui.table_next_row();
            cur_category!(self, table(ui));
            tbl.end();
        }

        ui.same_line();
        ui.child_window("Object Data").build(|| {
            cur_category!(self, object_data(ui));
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct Options {
    case_sensitive: bool,
    search_name: bool,
    search_id: bool,
    search_display_name: bool,
}

impl Options {
    pub fn new() -> Self {
        Self {
            case_sensitive: false,
            search_name: true,
            search_id: false,
            search_display_name: true,
        }
    }
}

#[derive(Debug, Clone)]
struct ItemsCategory {
    items: Vec<Item>,
    options: ItemsOptions,
    selected: Option<usize>,
}

impl ItemsCategory {
    pub fn new() -> Self {
        Self { items: Vec::new(), options: ItemsOptions::new(), selected: None }
    }

    fn columns(&self) -> usize {
        2
    }

    fn table(&mut self, ui: &Ui) {
        for (i, item) in self.items.iter_mut().enumerate() {
            ui.table_set_column_index(0);

            if ui
                .selectable_config(&format!("##selectable{i}"))
                .span_all_columns(true)
                .selected(self.selected.is_some_and(|x| x == i))
                .build()
            {
                self.selected = Some(i);
            }
            ui.same_line();
            if let Some(display_name) = item.display_name.as_mut() {
                ui.text_wrapped(display_name);
            }
            ui.table_next_column();

            ui.text_wrapped(&item.name);
            ui.table_next_column();
        }
    }

    fn object_data(&mut self, ui: &Ui) {
        if let Some(selected_item) = self.selected {
            self.items[selected_item].render(ui);
        }
    }

    fn render_options(&mut self, ui: &Ui) {
        ui.checkbox("Search Description", &mut self.options.search_desc);
    }

    fn search(&mut self, text: &str, opts: &Options) {
        self.items.clear();
        let lowercase = text.to_lowercase();
        let text = if opts.case_sensitive { text } else { &lowercase };
        let pred = if opts.case_sensitive {
            |string: &str, text: &str| string.contains(text)
        } else {
            |string: &str, text: &str| string.to_lowercase().contains(text)
        };
        self.items.extend(
            templates()
                .filter_map(|x| match x {
                    SearchItem::Item(x) => Some(x),
                    _ => None,
                })
                .filter(|x| {
                    opts.search_name && pred(&x.name, text)
                        || opts.search_id && pred(&x.id, text)
                        || opts.search_display_name
                            && x.display_name.as_ref().is_some_and(|x| pred(x, text))
                        || self.options.search_desc
                            && x.desc.as_ref().is_some_and(|x| pred(x, text))
                }),
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct ItemsOptions {
    search_desc: bool,
}

impl ItemsOptions {
    pub fn new() -> Self {
        Self { search_desc: false }
    }
}

#[derive(Debug, Clone)]
struct OtherCategory {
    items: Vec<Other>,
    selected: Option<usize>,
}

impl OtherCategory {
    pub fn new() -> Self {
        Self { items: Vec::new(), selected: None }
    }

    fn columns(&self) -> usize {
        2
    }

    fn table(&mut self, ui: &Ui) {
        for (i, item) in self.items.iter_mut().enumerate() {
            ui.table_set_column_index(0);

            if ui
                .selectable_config(&format!("##selectable{i}"))
                .span_all_columns(true)
                .selected(self.selected.is_some_and(|x| x == i))
                .build()
            {
                self.selected = Some(i);
            }
            ui.same_line();
            if let Some(display_name) = item.display_name.as_mut() {
                ui.text_wrapped(display_name);
            }

            ui.table_next_column();
            ui.text_wrapped(&item.name);
            ui.table_next_column();
        }
    }

    fn object_data(&mut self, ui: &Ui) {
        if let Some(selected_item) = self.selected {
            self.items[selected_item].render(ui);
        }
    }

    fn render_options(&mut self, _ui: &Ui) {}

    fn search(&mut self, text: &str, opts: &Options) {
        self.items.clear();
        let lowercase = text.to_lowercase();
        let text = if opts.case_sensitive { text } else { &lowercase };
        let pred = if opts.case_sensitive {
            |string: &str, text: &str| string.contains(text)
        } else {
            |string: &str, text: &str| string.to_lowercase().contains(text)
        };
        self.items.extend(
            templates()
                .filter_map(|x| match x {
                    SearchItem::Other(x) => Some(x),
                    _ => None,
                })
                .filter(|x| {
                    opts.search_name && pred(&x.name, text)
                        || opts.search_id && pred(&x.id, text)
                        || opts.search_display_name
                            && x.display_name.as_ref().is_some_and(|x| pred(x, text))
                }),
        );
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

#[derive(Debug, Clone)]
pub(crate) struct Item {
    name: String,
    id: String,
    display_name: Option<String>,
    desc: Option<String>,
    give_amount: i32,
}

impl From<&ItemTemplate> for Item {
    fn from(value: &ItemTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.try_into().unwrap();
        let display_name = (*value.display_name).try_into().ok();
        let desc = (*value.description).try_into().ok();

        Self { name, id, display_name, desc, give_amount: 1 }
    }
}

impl Item {
    pub fn render(&mut self, ui: &Ui) {
        if let Some(tbl) = ui.begin_table("obj_data_tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            ui.text("GUID");
            ui.table_next_column();
            ui.text_wrapped(&self.id);
            ui.table_next_column();

            ui.text("Name");
            ui.table_next_column();
            ui.text_wrapped(&self.name);
            ui.table_next_column();

            if let Some(display_name) = &self.display_name {
                ui.text("Display Name");
                ui.table_next_column();
                ui.text_wrapped(display_name);
                ui.table_next_column();
            }

            if let Some(desc) = &self.desc {
                ui.text("Description");
                ui.table_next_column();
                ui.text_wrapped(desc);
                ui.table_next_column();
            }

            tbl.end();

            ui.input_int("Amount", &mut self.give_amount).build();
            if self.give_amount < 1 {
                self.give_amount = 1;
            }

            if ui.button("Give") {
                if let Err(err) = give_item(&self.id, self.give_amount) {
                    err!("failed to give item: {err}");
                };
            }
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
    pub fn render(&mut self, ui: &Ui) {
        if let Some(tbl) = ui.begin_table("obj_data_tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            ui.text("GUID");
            ui.table_next_column();
            ui.text_wrapped(&self.id);
            ui.table_next_column();

            ui.text("Name");
            ui.table_next_column();
            ui.text_wrapped(&self.name);
            ui.table_next_column();

            if let Some(display_name) = &self.display_name {
                ui.text("Display Name");
                ui.table_next_column();
                ui.text_wrapped(display_name);
                ui.table_next_column();
            }

            tbl.end();
        }
    }
}

fn templates() -> impl Iterator<Item = SearchItem> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| SearchItem::from(Template::from(x.value.as_ref())))
}

fn give_item(uuid: &str, amount: i32) -> anyhow::Result<()> {
    osi_fn!(TemplateAddTo, uuid, get_host_character()?, amount, 0)?;
    Ok(())
}

fn get_host_character() -> anyhow::Result<osiris::Value> {
    Ok(osi_fn!(GetHostCharacter)?.unwrap())
}
