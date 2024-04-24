use std::{borrow::Cow, cmp::Ordering};

use imgui::{MouseButton, TableColumnFlags, TableFlags, TableSortDirection, Ui};

use self::{
    functions::{Function, FunctionCategory},
    items::{Item, ItemCategory},
    other::Other,
    spells::Spell,
};
use crate::{
    game_definitions::{
        FixedString, GameObjectTemplate, LSStringView, OverrideableProperty, STDString,
        SpellPrototype, Template,
    },
    globals::Globals,
};

mod functions;
mod items;
mod other;
mod spells;

macro_rules! choose_category {
    ($ident:ident, $($tt:tt)*) => {
        match $ident.cur_category {
            0 => $ident.items.$($tt)*,
            // 1 => $ident.spells.$($tt)*,
            2 => $ident.functions.$($tt)*,
            3 => $ident.other.$($tt)*,
            _ => $ident.other.$($tt)*,
        }
    };
}

pub(crate) struct Search {
    reclaim_focus: bool,
    cur_category: usize,
    text: String,
    options: Options,
    functions: ObjectTable<FunctionCategory>,
    items: ObjectTable<ItemCategory>,
    other: ObjectTable<GameObjectTemplateCategory>,
    // spells: ObjectTable<'a, Spell, SpellSource>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            reclaim_focus: true,
            cur_category: 0,
            text: String::new(),
            options: Options::default(),
            functions: ObjectTable::default(),
            items: ObjectTable::default(),
            other: ObjectTable::default(),
        }
    }
}

impl Search {
    pub fn render(&mut self, ui: &Ui) {
        macro_rules! cur_category {
            ($($tt:tt)*) => {
                choose_category!(self, $($tt)*)
            };
        }

        ui.text("Object Category");
        if ui.combo(
            "##object-category-combo",
            &mut self.cur_category,
            &["Items", "Spells", "Osiris Functions", "Other"],
            |x| Cow::from(*x),
        ) && self.text.is_empty()
            && cur_category!(items.len() == 0)
        {
            self.search();
        }

        if let Some(node) = ui.tree_node("Search Options") {
            if ui.checkbox("Case Sensitive", &mut self.options.case_sensitive) {
                self.search();
            };
            if let Some(node) = ui.tree_node("Search Fields") {
                if cur_category!(draw_options(ui)) {
                    self.search();
                }
                node.pop();
            }
            node.pop();
        }
        ui.separator();

        ui.text("Search");
        if self.cur_category == 2 {
            ui.same_line();
            ui.text_disabled("(?)");
            if ui.is_item_hovered() {
                ui.tooltip(|| ui.text("Only works when a save game is loaded"));
            }
        }
        if self.reclaim_focus {
            ui.set_keyboard_focus_here();
            self.reclaim_focus = false;
        }
        if ui.input_text("<<", &mut self.text).enter_returns_true(true).build() {
            self.search();
        }
        ui.set_item_default_focus();
        ui.same_line();
        if ui.button("Search") {
            self.search();
        }

        if let Some(body) = ui.begin_table_with_flags("body-tbl", 2, TableFlags::RESIZABLE) {
            ui.table_next_row();
            ui.table_set_column_index(0);
            cur_category!(draw_table(ui));
            ui.table_next_column();
            cur_category!(draw_details(ui));
            body.end();
        }
    }

    fn search(&mut self) {
        macro_rules! cur_category {
            ($($tt:tt)*) => {
                choose_category!(self, $($tt)*)
            };
        }

        cur_category!(search(&self.text, &self.options));
        self.reclaim_focus = true;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Options {
    case_sensitive: bool,
}

pub(crate) trait TableItem {
    fn columns() -> Box<[TableColumn]>;
    fn draw(&self, ui: &Ui, i: usize);
    fn search_str(&self, i: usize) -> String;
    fn compare(&self, other: &Self, i: usize) -> Ordering;
}

#[derive(Debug, Clone)]
pub(crate) struct TableColumn {
    name: String,
    shown: bool,
    included_in_search: bool,
}

impl TableColumn {
    pub fn new(name: impl AsRef<str>, shown: bool, included_in_search: bool) -> Self {
        Self { name: name.as_ref().to_string(), included_in_search, shown }
    }
}

pub(crate) trait TableItemCategory: Default {
    type Item: TableItem;

    fn source() -> impl Iterator<Item = Self::Item>;
    fn filter(&self, _item: &Self::Item) -> bool {
        true
    }
    fn draw_options(&mut self, _ui: &Ui) -> bool {
        false
    }
    fn draw_actions(&mut self, _ui: &Ui, _item: &mut Self::Item) {}
}

#[derive(Default)]
pub(crate) struct GameObjectTemplateCategory;
impl TableItemCategory for GameObjectTemplateCategory {
    type Item = &'static GameObjectTemplate;

    fn source() -> impl Iterator<Item = Self::Item> {
        templates()
    }
}

pub(crate) struct ObjectTable<T: TableItemCategory> {
    pub category: T,
    pub columns: Box<[TableColumn]>,
    pub items: Vec<T::Item>,
    pub selected: Option<usize>,
    pub page: usize,
    pub items_per_page: usize,
}

impl<T: TableItemCategory> Default for ObjectTable<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            columns: T::Item::columns(),
            category: T::default(),
            selected: None,
            page: 0,
            items_per_page: 1000,
        }
    }
}

impl<T: TableItemCategory> ObjectTable<T> {
    fn search(&mut self, string: &str, opts: &Options) {
        self.selected.take();
        self.items.clear();
        let mut search = |string: &str, pred: fn(&str, &str) -> bool| {
            self.items.extend(T::source().filter(|x| {
                self.columns.iter().enumerate().filter(|(_, col)| col.included_in_search).any(
                    |(i, _)| {
                        let item = x.search_str(i);
                        pred((*item).as_ref(), string) && self.category.filter(x)
                    },
                )
            }))
        };

        if opts.case_sensitive {
            search(string, |text, string| text.contains(string))
        } else {
            let string = string.to_lowercase();
            search(&string, |text, string| text.to_lowercase().contains(string))
        };
    }

    fn draw_table(&mut self, ui: &Ui) {
        if self.items.len() > self.items_per_page {
            let first_item_index = self.page * self.items_per_page;
            ui.text(format!(
                "found {} entries, showing {} - {}",
                self.items.len(),
                first_item_index + 1,
                first_item_index + self.items_per_page.min(self.items.len() - first_item_index)
            ));
            let max_pages = self.items.len().saturating_sub(1) / self.items_per_page;
            if ui.button("<") {
                self.page = self.page.saturating_sub(1);
            }
            ui.same_line();
            ui.text(format!("{} of {}", self.page + 1, max_pages + 1));
            ui.same_line();
            if ui.button(">") {
                self.page = (self.page + 1).min(max_pages);
            }
        } else {
            ui.text(format!("found {} entries", self.items.len()));
        }
        ui.text("Items per page");
        ui.same_line();
        let mut items_per_page = self.items_per_page as i32;
        ui.input_int("##items-per-page", &mut items_per_page).step(100).build();
        items_per_page = items_per_page.max(1);
        self.items_per_page = items_per_page as _;

        if let Some(tbl) = ui.begin_table_with_flags(
            "items-tbl",
            self.columns.len(),
            TableFlags::SCROLL_Y
                | TableFlags::RESIZABLE
                | TableFlags::REORDERABLE
                | TableFlags::HIDEABLE
                | TableFlags::SORTABLE,
        ) {
            for field in self.columns.iter() {
                ui.table_setup_column_with(imgui::TableColumnSetup {
                    name: field.name.as_str(),
                    flags: if field.shown {
                        TableColumnFlags::default()
                    } else {
                        TableColumnFlags::DEFAULT_HIDE
                    },
                    ..Default::default()
                });
            }
            ui.table_headers_row();
            ui.table_next_row();
            if let Some(specs) = ui.table_sort_specs_mut() {
                specs.conditional_sort(|specs| {
                    if let Some(specs) = specs.iter().next() {
                        match specs.sort_direction() {
                            Some(TableSortDirection::Ascending) => {
                                self.items.sort_by(|a, b| a.compare(b, specs.column_idx()))
                            }
                            Some(TableSortDirection::Descending) => self
                                .items
                                .sort_by(|a, b| a.compare(b, specs.column_idx()).reverse()),
                            None => (),
                        }
                    }
                });
            }

            for (i, item) in self
                .items
                .iter()
                .enumerate()
                .skip(self.page * self.items_per_page)
                .take(self.items_per_page)
            {
                ui.table_set_column_index(0);

                item.draw(ui, 0);
                let mut max_height = ui.item_rect_size()[1];
                for j in 1..self.columns.len() {
                    ui.table_next_column();
                    item.draw(ui, j);
                    max_height = ui.item_rect_size()[1].max(max_height);
                }

                for j in 0..self.columns.len() {
                    if ui.table_set_column_index(j) {
                        if ui
                            .selectable_config(&format!("##selectable{i}"))
                            .span_all_columns(true)
                            .selected(self.selected.is_some_and(|x| x == i))
                            .size([0.0, max_height])
                            .build()
                        {
                            self.selected.replace(i);
                        }
                        break;
                    }
                }
                ui.table_next_row();
            }
            tbl.end();
        }
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        let mut changed = false;
        for col in self.columns.iter_mut() {
            changed |= ui.checkbox(&col.name, &mut col.included_in_search);
        }
        changed || self.category.draw_options(ui)
    }

    fn draw_details(&mut self, ui: &Ui) {
        if let Some(selected) = self.selected {
            let item = &mut self.items[selected];
            if let Some(tbl) = ui.begin_table_with_flags("obj-data-tbl", 2, TableFlags::RESIZABLE) {
                ui.table_next_row();
                ui.table_set_column_index(0);

                for (i, col) in self.columns.iter().enumerate() {
                    ui.text(&col.name);
                    ui.table_next_column();
                    item.draw(ui, i);
                    if ui.is_item_hovered() {
                        if ui.is_mouse_clicked(MouseButton::Right) {
                            ui.set_clipboard_text(item.search_str(i));
                        }
                        if ui.clipboard_text().is_some_and(|x| x == item.search_str(i)) {
                            ui.tooltip(|| ui.text("Copied!"));
                        } else {
                            ui.tooltip(|| ui.text("Right click to copy"));
                        }
                    }
                    ui.table_next_column();
                }

                tbl.end();

                self.category.draw_actions(ui, item);
            }
        }
    }
}

pub(crate) trait TableValue {
    fn search_str(&self) -> String;
    fn draw(&self, ui: &Ui);
    fn compare(&self, other: &Self) -> Ordering;
}

macro_rules! table_value_primitive {
    ($type:ty) => {
        impl TableValue for $type {
            fn search_str(&self) -> String {
                self.to_string()
            }

            fn draw(&self, ui: &Ui) {
                ui.text_wrapped(self.to_string());
            }

            fn compare(&self, other: &Self) -> Ordering {
                self.cmp(other)
            }
        }
    };
}

table_value_primitive!(bool);
table_value_primitive!(u8);
table_value_primitive!(u16);
table_value_primitive!(u32);
table_value_primitive!(u64);
table_value_primitive!(usize);
table_value_primitive!(i8);
table_value_primitive!(i16);
table_value_primitive!(i32);
table_value_primitive!(i64);
table_value_primitive!(isize);

impl TableValue for &str {
    fn search_str(&self) -> String {
        self.to_string()
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self)
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

impl TableValue for String {
    fn search_str(&self) -> String {
        self.clone()
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self);
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

impl<T: TableValue + Ord> TableValue for Option<T> {
    fn search_str(&self) -> String {
        self.as_ref().map(|x| x.search_str()).unwrap_or_default()
    }

    fn draw(&self, ui: &Ui) {
        if let Some(x) = self {
            x.draw(ui)
        }
    }

    fn compare(&self, other: &Self) -> Ordering {
        let ord = self.cmp(other);
        if self.is_some() && other.is_none() || self.is_none() && other.is_some() {
            ord.reverse()
        } else {
            ord
        }
    }
}

impl<T: TableValue> TableValue for OverrideableProperty<T> {
    fn search_str(&self) -> String {
        self.value.search_str()
    }

    fn draw(&self, ui: &Ui) {
        self.value.draw(ui)
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.value.compare(&other.value)
    }
}

impl TableValue for FixedString {
    fn search_str(&self) -> String {
        self.get().search_str()
    }

    fn draw(&self, ui: &Ui) {
        self.get().draw(ui)
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.get().compare(&other.get())
    }
}

impl TableValue for LSStringView<'_> {
    fn search_str(&self) -> String {
        self.as_str().search_str()
    }

    fn draw(&self, ui: &Ui) {
        self.as_str().draw(ui)
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.as_str().compare(&other.as_str())
    }
}

impl TableValue for STDString {
    fn search_str(&self) -> String {
        self.as_str().search_str()
    }

    fn draw(&self, ui: &Ui) {
        self.as_str().draw(ui)
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.as_str().compare(&other.as_str())
    }
}

#[derive(Debug, Clone)]
enum SearchItem {
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
    Spell(Spell),
    Function(Function),
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

impl From<&SpellPrototype> for SearchItem {
    fn from(value: &SpellPrototype) -> Self {
        Self::Spell(value.into())
    }
}

fn templates2() -> impl Iterator<Item = SearchItem> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| Template::from(x.value.as_ref()).into())
}

fn templates<'a>() -> impl Iterator<Item = &'a GameObjectTemplate> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| x.value.as_ref())
}
