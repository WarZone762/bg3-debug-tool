use std::{borrow::Cow, cmp::Ordering, marker::PhantomData};

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

pub(crate) struct ObjectTable<T: ObjectTableItem> {
    pub fields: Box<[Box<dyn TableValueGetter<T>>]>,
    pub items: Vec<T>,
    pub selected: Option<usize>,
    pub options: T::Options,
    pub actions: T::ActionMenu,
}

impl<T: ObjectTableItem> Default for ObjectTable<T> {
    fn default() -> Self {
        Self {
            fields: T::fields(),
            items: Vec::new(),
            selected: None,
            options: T::Options::default(),
            actions: T::ActionMenu::default(),
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
                    pred((*item).as_ref(), string) && x.filter(&self.options)
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

    fn draw_details(&mut self, ui: &Ui) {
        if let Some(selected) = self.selected {
            let item = &mut self.items[selected];
            if let Some(tbl) = ui.begin_table_with_flags("obj-data-tbl", 2, TableFlags::RESIZABLE) {
                ui.table_next_row();
                ui.table_set_column_index(0);

                for field in self.fields.iter() {
                    ui.text(field.name());
                    ui.table_next_column();
                    field.draw(ui, item);
                    if ui.is_item_hovered() {
                        if ui.is_mouse_clicked(MouseButton::Right) {
                            ui.set_clipboard_text(field.search_str(item).as_ref());
                        }
                        if ui
                            .clipboard_text()
                            .is_some_and(|x| x == (*field.search_str(item)).as_ref())
                        {
                            ui.tooltip(|| ui.text("Copied!"));
                        } else {
                            ui.tooltip(|| ui.text("Right click to copy"));
                        }
                    }
                    ui.table_next_column();
                }

                tbl.end();

                self.actions.draw(ui, item);
            }
        }
    }
}

pub(crate) trait Getter<'a, T: ObjectTableItem + 'a> {
    type Output: TableValue + 'a;
    fn get(&self, ctx: &'a T) -> Self::Output;
}

impl<'a, T: ObjectTableItem + 'a, F: Fn(&'a T) -> R, R: TableValue + 'a> Getter<'a, T> for F {
    type Output = R;

    fn get(&self, ctx: &'a T) -> Self::Output {
        (self)(ctx)
    }
}

pub(crate) struct ObjectField<
    T: ObjectTableItem,
    // V: TableValue + 'static,
    G: for<'a> Getter<'a, T>,
> {
    pub name: String,
    pub included_in_search: bool,
    pub getter: G,
    marker: PhantomData<T>,
}

impl<T: ObjectTableItem, G: for<'a> Getter<'a, T>> ObjectField<T, G> {
    pub fn get<'a>(&self, item: &'a T) -> impl TableValue + 'a {
        self.getter.get(item)
    }

    pub fn define<'a>(
        name: impl AsRef<str>,
        included_in_search: bool,
        getter: G,
    ) -> Box<dyn TableValueGetter<T> + 'a>
    where
        T: 'a,
        G: 'a,
    {
        Box::new(Self {
            name: name.as_ref().into(),
            included_in_search,
            getter,
            marker: PhantomData,
        })
    }
}

pub(crate) trait TableValueGetter<T: ?Sized> {
    fn name(&self) -> &str;
    fn included_in_search(&self) -> bool;
    fn included_in_search_set(&mut self, value: bool);
    fn search_str<'a>(&self, item: &'a T) -> Box<dyn AsRef<str> + 'a>;
    fn draw(&self, ui: &Ui, item: &T);
    fn compare(&self, a: &T, b: &T) -> Ordering;
}

impl<T: ObjectTableItem, G: for<'a> Getter<'a, T>> TableValueGetter<T> for ObjectField<T, G> {
    fn name(&self) -> &str {
        &self.name
    }

    fn included_in_search(&self) -> bool {
        self.included_in_search
    }

    fn included_in_search_set(&mut self, value: bool) {
        self.included_in_search = value;
    }

    fn search_str<'a>(&self, item: &'a T) -> Box<dyn AsRef<str> + 'a> {
        Box::new(self.get(item).search_str())
    }

    fn draw(&self, ui: &Ui, item: &T) {
        self.get(item).draw(ui);
    }

    fn compare(&self, a: &T, b: &T) -> Ordering {
        self.get(a).compare(&self.get(b))
    }
}

pub(crate) trait TableValue: Ord {
    fn search_str<'a>(self) -> impl AsRef<str> + 'a
    where
        Self: 'a;
    fn draw(&self, ui: &Ui);
    fn compare(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

impl TableValue for &str {
    fn search_str<'a>(self) -> impl AsRef<str> + 'a
    where
        Self: 'a,
    {
        self
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self);
    }
}

impl TableValue for &String {
    fn search_str<'a>(self) -> impl AsRef<str> + 'a
    where
        Self: 'a,
    {
        self
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self);
    }
}

impl TableValue for String {
    fn search_str<'a>(self) -> impl AsRef<str> + 'a
    where
        Self: 'a,
    {
        self
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self);
    }
}

impl TableValue for Option<&str> {
    fn search_str<'a>(self) -> impl AsRef<str> + 'a
    where
        Self: 'a,
    {
        self.unwrap_or("")
    }

    fn draw(&self, ui: &Ui) {
        if let Some(text) = self {
            ui.text_wrapped(text)
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

pub(crate) trait ObjectTableItem: Sized {
    type ActionMenu: TableItemActions<Self>;
    type Options: TableOptions;

    fn fields() -> Box<[Box<dyn TableValueGetter<Self>>]>;
    fn source() -> impl Iterator<Item = Self>;
    fn filter(&self, _opts: &Self::Options) -> bool {
        true
    }
}

pub(crate) trait TableItemActions<T>: Default {
    fn draw(&mut self, ui: &Ui, item: &mut T);
}

impl<T> TableItemActions<T> for () {
    fn draw(&mut self, _ui: &Ui, _item: &mut T) {}
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

fn templates() -> impl Iterator<Item = SearchItem> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| Template::from(x.value.as_ref()).into())
}
