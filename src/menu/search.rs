use std::{borrow::Cow, cmp::Ordering};

use imgui::{MouseButton, TableFlags, TableSortDirection, Ui};

use self::{functions::Function, items::Item, other::Other, spells::Spell};
use crate::{
    game_definitions::{SpellPrototype, Template},
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
            1 => $ident.spells.$($tt)*,
            2 => $ident.functions.$($tt)*,
            3 => $ident.other.$($tt)*,
            _ => $ident.other.$($tt)*,
        }
    };
}

pub(crate) struct Search {
    cur_category: usize,
    text: String,
    options: Options,
    items: ObjectTable<Item>,
    spells: ObjectTable<Spell>,
    functions: ObjectTable<Function>,
    other: ObjectTable<Other>,
    reclaim_focus: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            reclaim_focus: true,
            cur_category: 0,
            text: String::new(),
            options: Options::default(),
            items: ObjectTable::default(),
            spells: ObjectTable::default(),
            functions: ObjectTable::default(),
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
        ui.combo(
            "##object-category-combo",
            &mut self.cur_category,
            &["Items", "Spells", "Osiris Functions", "Other"],
            |x| Cow::from(*x),
        );

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

        ui.text(format!("found {} entries", cur_category!(items.len())));

        if let Some(body) = ui.begin_table_with_flags("body-tbl", 2, TableFlags::RESIZABLE) {
            ui.table_next_row();
            ui.table_set_column_index(0);
            cur_category!(draw_table(ui));
            ui.table_next_column();
            if let Some(selected_item) = cur_category!(selected) {
                cur_category!(items[selected_item].render(ui));
            }
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

pub(crate) struct ObjectTable<T: ObjectTableItem> {
    pub fields: Box<[Box<dyn TableValueGetter<T>>]>,
    pub items: Vec<T>,
    pub selected: Option<usize>,
    pub options: T::Options,
}

impl<T: ObjectTableItem> Default for ObjectTable<T> {
    fn default() -> Self {
        Self {
            fields: T::fields(),
            items: Vec::new(),
            selected: None,
            options: T::Options::default(),
        }
    }
}

impl<T: ObjectTableItem> ObjectTable<T> {
    fn search(&mut self, string: &str, opts: &Options) {
        self.selected.take();
        self.items.clear();
        let mut search = |string: &str, pred: fn(&str, &str) -> bool| {
            self.items.extend(T::source().filter(|x| {
                self.fields.iter().filter(|field| field.included_in_search()).any(|field| {
                    let item = field.search_str(x);
                    pred(item, string) && x.filter(&self.options)
                })
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
        if let Some(tbl) = ui.begin_table_with_flags(
            "items-tbl",
            self.fields.len(),
            TableFlags::SCROLL_Y
                | TableFlags::RESIZABLE
                | TableFlags::REORDERABLE
                | TableFlags::HIDEABLE
                | TableFlags::SORTABLE,
        ) {
            for field in self.fields.iter() {
                ui.table_setup_column(field.name());
            }
            ui.table_headers_row();
            ui.table_next_row();
            if let Some(specs) = ui.table_sort_specs_mut() {
                specs.conditional_sort(|specs| {
                    if let Some(specs) = specs.iter().next() {
                        match specs.sort_direction() {
                            Some(TableSortDirection::Ascending) => self.items.sort_by(|a, b| {
                                let field = &self.fields[specs.column_idx()];
                                field.compare(a, b)
                            }),
                            Some(TableSortDirection::Descending) => self.items.sort_by(|a, b| {
                                let field = &self.fields[specs.column_idx()];
                                field.compare(a, b).reverse()
                            }),
                            None => (),
                        }
                    }
                });
            }

            for (i, item) in self.items.iter().enumerate() {
                ui.table_set_column_index(0);

                self.fields[0].draw(ui, item);
                let mut max_height = ui.item_rect_size()[1];
                for field in &self.fields[1..] {
                    ui.table_next_column();
                    field.draw(ui, item);
                    max_height = ui.item_rect_size()[1].max(max_height);
                }
                ui.same_line();
                if ui
                    .selectable_config(&format!("##selectable{i}"))
                    .span_all_columns(true)
                    .selected(self.selected.is_some_and(|x| x == i))
                    .size([0.0, max_height])
                    .build()
                {
                    self.selected.replace(i);
                }
                ui.table_next_column();
            }
            tbl.end();
        }
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        let mut changed = false;
        for field in self.fields.iter_mut() {
            let mut new = field.included_in_search();
            changed |= ui.checkbox(field.name(), &mut new);
            field.included_in_search_set(new);
        }
        changed || self.options.draw(ui)
    }
}

pub(crate) struct ObjectField<T: ObjectTableItem + 'static, V: TableValue + 'static> {
    pub name: String,
    pub included_in_search: bool,
    pub getter: fn(&T) -> &V,
}

impl<T: ObjectTableItem, V: TableValue> ObjectField<T, V> {
    pub fn getter(
        name: impl AsRef<str>,
        included_in_search: bool,
        getter: fn(&T) -> &V,
    ) -> Box<dyn TableValueGetter<T>> {
        Box::new(Self { name: name.as_ref().into(), included_in_search, getter })
    }
}

pub(crate) trait TableValueGetter<T: ?Sized> {
    fn name(&self) -> &str;
    fn included_in_search(&self) -> bool;
    fn included_in_search_set(&mut self, value: bool);
    fn search_str<'a>(&self, item: &'a T) -> &'a str;
    fn draw(&self, ui: &Ui, item: &T);
    fn compare(&self, a: &T, b: &T) -> Ordering;
}

impl<T: ObjectTableItem, V: TableValue> TableValueGetter<T> for ObjectField<T, V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn included_in_search(&self) -> bool {
        self.included_in_search
    }

    fn included_in_search_set(&mut self, value: bool) {
        self.included_in_search = value;
    }

    fn search_str<'a>(&self, item: &'a T) -> &'a str {
        (self.getter)(item).search_str()
    }

    fn draw(&self, ui: &Ui, item: &T) {
        (self.getter)(item).draw(ui);
    }

    fn compare(&self, a: &T, b: &T) -> Ordering {
        (self.getter)(a).compare((self.getter)(b))
    }
}

pub(crate) trait TableValue {
    fn search_str(&self) -> &str;
    fn draw(&self, ui: &Ui);
    fn compare(&self, other: &Self) -> Ordering;
}

impl TableValue for &str {
    fn search_str(&self) -> &str {
        self
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self);
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

impl TableValue for String {
    fn search_str(&self) -> &str {
        self
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self);
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

impl TableValue for Option<String> {
    fn search_str(&self) -> &str {
        self.as_deref().unwrap_or("")
    }

    fn draw(&self, ui: &Ui) {
        if let Some(text) = self {
            ui.text_wrapped(text)
        }
    }

    fn compare(&self, other: &Self) -> Ordering {
        option_cmp_reverse(self, other)
    }
}

pub(crate) trait ObjectTableItem: Sized {
    type Options: TableOptions;

    fn fields() -> Box<[Box<dyn TableValueGetter<Self>>]>;
    fn source() -> impl Iterator<Item = Self>;
    fn filter(&self, _opts: &Self::Options) -> bool {
        true
    }
}

pub(crate) trait TableOptions: Default {
    fn draw(&mut self, ui: &Ui) -> bool;
}

impl TableOptions for () {
    fn draw(&mut self, _ui: &Ui) -> bool {
        false
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

fn option_cmp_reverse<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    let ord = a.cmp(b);
    if a.is_some() && b.is_none() || a.is_none() && b.is_some() {
        ord.reverse()
    } else {
        ord
    }
}

fn object_data_tbl(ui: &Ui, f: impl FnOnce(&dyn Fn(&str, &str))) {
    if let Some(tbl) = ui.begin_table_with_flags("obj-data-tbl", 2, TableFlags::RESIZABLE) {
        ui.table_next_row();
        ui.table_set_column_index(0);

        let row = |name: &str, text: &str| {
            ui.text(name);
            ui.table_next_column();
            ui.text_wrapped(text);
            if ui.is_item_hovered() {
                if ui.is_mouse_clicked(MouseButton::Right) {
                    ui.set_clipboard_text(text);
                }
                if ui.clipboard_text().is_some_and(|x| x == text) {
                    ui.tooltip(|| ui.text("Copied!"));
                } else {
                    ui.tooltip(|| ui.text("Right click to copy"));
                }
            }
            ui.table_next_column();
        };

        f(&row);

        tbl.end();
    }
}

fn templates() -> impl Iterator<Item = SearchItem> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| Template::from(x.value.as_ref()).into())
}
