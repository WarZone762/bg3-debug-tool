use std::borrow::Cow;

use imgui::{MouseButton, TableFlags, Ui};

use crate::{
    err,
    game_definitions::{EoCGameObjectTemplate, GameObjectTemplate, ItemTemplate, Template},
    globals::Globals,
    osi_fn,
    wrappers::osiris,
};

macro_rules! choose_category {
    ($ident:ident, $($tt:tt)*) => {
        match $ident.cur_category {
            0 => $ident.items.$($tt)*,
            1 => $ident.other.$($tt)*,
            _ => $ident.items.$($tt)*,
        }
    };
}

#[derive(Debug, Clone)]
pub(crate) struct Search {
    cur_category: usize,
    text: String,
    options: Options,
    items: ItemsCategory,
    other: OtherCategory,
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
        macro_rules! cur_category {
            ($($tt:tt)*) => {
                choose_category!(self, $($tt)*)
            };
        }

        if let Some(node) = ui.tree_node("Search Options") {
            if ui.checkbox("Case Sensitive", &mut self.options.case_sensitive) {
                self.search();
            };
            if let Some(node) = ui.tree_node("Search Fields") {
                if ui.checkbox("Search Name", &mut self.options.search_name)
                    || ui.checkbox("Search GUID", &mut self.options.search_id)
                    || ui.checkbox("Search Display Name", &mut self.options.search_display_name)
                    || cur_category!(draw_options(ui))
                {
                    self.search();
                }
                node.pop();
            }
            node.pop();
        }
        ui.combo("Object Category", &mut self.cur_category, &["Items", "Other"], |x| Cow::from(*x));
        ui.separator();
        ui.text("Search");
        if ui.input_text("<<", &mut self.text).enter_returns_true(true).build() {
            self.search();
        }

        cur_category!(draw_table(ui));

        ui.same_line();
        ui.child_window("Object Data").build(|| {
            if let Some(selected_item) = cur_category!(selected) {
                cur_category!(items[selected_item].render(ui));
            }
        });
    }

    fn search(&mut self) {
        macro_rules! cur_category {
            ($($tt:tt)*) => {
                choose_category!(self, $($tt)*)
            };
        }

        cur_category!(search(&self.text, &self.options));
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

    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &self.options, &mut self.selected)
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &self.items, &mut self.selected);
    }
}

impl Category for ItemsCategory {
    type Item = Item;
    type Options = ItemsOptions;

    const COLS: usize = 2;

    fn draw_table_row(ui: &Ui, item: &Self::Item) {
        if let Some(display_name) = item.display_name.as_ref() {
            ui.text_wrapped(display_name);
        }
        ui.table_next_column();

        ui.text_wrapped(&item.name);
        ui.table_next_column();
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        ui.checkbox("Search Description", &mut self.options.search_desc)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Item(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool {
        opts.search_name && pred(&item.name)
            || opts.search_id && pred(&item.id)
            || opts.search_display_name && item.display_name.as_deref().is_some_and(&pred)
            || self_opts.search_desc && item.desc.as_deref().is_some_and(&pred)
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

    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &(), &mut self.selected);
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &self.items, &mut self.selected);
    }
}

impl Category for OtherCategory {
    type Item = Other;
    type Options = ();

    const COLS: usize = 2;

    fn draw_table_row(ui: &Ui, item: &Self::Item) {
        if let Some(display_name) = item.display_name.as_ref() {
            ui.text_wrapped(display_name);
        }
        ui.table_next_column();

        ui.text_wrapped(&item.name);
        ui.table_next_column();
    }

    fn draw_options(&mut self, _ui: &Ui) -> bool {
        false
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Other(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        _self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool {
        opts.search_name && pred(&item.name)
            || opts.search_id && pred(&item.id)
            || opts.search_display_name && item.display_name.as_deref().is_some_and(pred)
    }
}

trait Category {
    const COLS: usize;
    type Item;
    type Options;

    fn draw_table_row(ui: &Ui, item: &Self::Item);
    fn draw_options(&mut self, ui: &Ui) -> bool;
    fn search_filter_map(item: SearchItem) -> Option<Self::Item>;
    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool;

    fn search_impl(
        items: &mut Vec<Self::Item>,
        text: &str,
        opts: &Options,
        self_opts: &Self::Options,
        selected: &mut Option<usize>,
    ) {
        selected.take();
        items.clear();
        if opts.case_sensitive {
            let pred = &|string: &str| string.contains(text);
            items.extend(
                templates()
                    .filter_map(Self::search_filter_map)
                    .filter(|x| Self::search_filter(x, opts, self_opts, pred)),
            )
        } else {
            let text = text.to_lowercase();
            let pred = &|string: &str| string.to_lowercase().contains(&text);
            items.extend(
                templates()
                    .filter_map(Self::search_filter_map)
                    .filter(|x| Self::search_filter(x, opts, self_opts, pred)),
            );
        }
    }

    fn draw_table_impl(ui: &Ui, items: &[Self::Item], selected: &mut Option<usize>) {
        let size = ui.window_size();
        if let Some(tbl) = ui.begin_table_with_sizing(
            "items-tbl",
            Self::COLS,
            TableFlags::SCROLL_Y,
            [size[0] * 0.5, -1.0],
            0.0,
        ) {
            ui.table_next_row();
            for (i, item) in items.iter().enumerate() {
                ui.table_set_column_index(0);

                if ui
                    .selectable_config(&format!("##selectable{i}"))
                    .span_all_columns(true)
                    .selected(selected.is_some_and(|x| x == i))
                    .build()
                {
                    selected.replace(i);
                }
                ui.same_line();

                Self::draw_table_row(ui, item);
            }
            tbl.end();
        }
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
        if let Some(tbl) = ui.begin_table("obj-data-tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            object_data_row(ui, "GUID", &self.id);
            object_data_row(ui, "Name", &self.name);
            if let Some(display_name) = &self.display_name {
                object_data_row(ui, "Display Name", display_name);
            }
            if let Some(desc) = &self.desc {
                object_data_row(ui, "Description", desc);
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
        if let Some(tbl) = ui.begin_table("obj-data-tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            object_data_row(ui, "GUID", &self.id);
            object_data_row(ui, "Name", &self.name);
            if let Some(display_name) = &self.display_name {
                object_data_row(ui, "Display Name", display_name);
            }

            tbl.end();
        }
    }
}

fn object_data_row(ui: &Ui, name: &str, text: &str) {
    ui.text(name);
    ui.table_next_column();
    ui.text_wrapped(text);
    copy_popup(ui, text);
    ui.table_next_column();
}

fn copy_popup(ui: &Ui, copy_text: &str) {
    if ui.is_item_hovered() {
        if ui.is_mouse_released(MouseButton::Right) {
            ui.set_clipboard_text(copy_text);
        }
        if ui.clipboard_text().is_some_and(|x| x == copy_text) {
            ui.tooltip(|| ui.text("Copied!"));
        } else {
            ui.tooltip(|| ui.text("Right click to copy"));
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
