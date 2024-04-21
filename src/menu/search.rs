use std::{borrow::Cow, cmp::Ordering};

use imgui::{MouseButton, TableFlags, TableSortDirection, Ui};

use self::{
    functions::{Function, FunctionCategory},
    items::{Item, ItemsCategory},
    other::{Other, OtherCategory},
    spells::{Spell, SpellCategory},
};
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

#[derive(Debug, Clone)]
pub(crate) struct Search {
    cur_category: usize,
    text: String,
    options: Options,
    items: ItemsCategory,
    spells: SpellCategory,
    functions: FunctionCategory,
    other: OtherCategory,
    reclaim_focus: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            reclaim_focus: true,
            cur_category: 0,
            text: String::new(),
            options: Options::default(),
            items: ItemsCategory::default(),
            spells: SpellCategory::default(),
            functions: FunctionCategory::default(),
            other: OtherCategory::default(),
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

        ui.combo(
            "Object Category",
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

trait Category<const N: usize> {
    const COLS: [&'static str; N];
    type Item;
    type Options;

    fn draw_table_row(ui: &Ui, item: &Self::Item, height_cb: impl FnMut());
    fn draw_options(&mut self, ui: &Ui) -> bool;
    fn search_filter_map(item: SearchItem) -> Option<Self::Item>;
    fn search_filter(item: &Self::Item, opts: &Self::Options, pred: impl Fn(&str) -> bool) -> bool;
    fn sort_pred(column: usize) -> fn(&Self::Item, &Self::Item) -> Ordering;

    fn search_iter() -> impl Iterator<Item = SearchItem>;

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
                Self::search_iter()
                    .filter_map(Self::search_filter_map)
                    .filter(|x| Self::search_filter(x, self_opts, pred)),
            )
        } else {
            let text = text.to_lowercase();
            let pred = &|string: &str| string.to_lowercase().contains(&text);
            items.extend(
                Self::search_iter()
                    .filter_map(Self::search_filter_map)
                    .filter(|x| Self::search_filter(x, self_opts, pred)),
            );
        }
    }

    fn draw_table_impl(ui: &Ui, items: &mut [Self::Item], selected: &mut Option<usize>) {
        // if let Some(tbl) = ui.begin_table_with_flags(
        //     "items-tbl",
        //     Self::COLS,
        //     TableFlags::SCROLL_Y | TableFlags::RESIZABLE,
        // ) {
        // let mut header_setup = Vec::with_capacity(Self::COLS);
        // for i in 0..Self::COLS {
        //     header_setup
        //         .push(TableColumnSetup { name: format!("column {i}"),
        // ..Default::default() }); }
        if let Some(tbl) = ui.begin_table_with_flags(
            "items-tbl",
            Self::COLS.len(),
            TableFlags::SCROLL_Y
                | TableFlags::RESIZABLE
                | TableFlags::REORDERABLE
                | TableFlags::HIDEABLE
                | TableFlags::SORTABLE,
        ) {
            for col in Self::COLS {
                ui.table_setup_column(col);
            }
            ui.table_headers_row();
            ui.table_next_row();
            if let Some(specs) = ui.table_sort_specs_mut() {
                specs.conditional_sort(|specs| {
                    if let Some(specs) = specs.iter().next() {
                        match specs.sort_direction() {
                            Some(TableSortDirection::Ascending) => {
                                items.sort_by(Self::sort_pred(specs.column_idx()))
                            }
                            Some(TableSortDirection::Descending) => items.sort_by(|a, b| {
                                Self::sort_pred(specs.column_idx())(a, b).reverse()
                            }),
                            None => (),
                        }
                    }
                });
            }

            for (i, item) in items.iter().enumerate() {
                ui.table_set_column_index(0);

                let mut max_height = 0.0;
                let height_cb = || {
                    max_height = ui.item_rect_size()[1].max(max_height);
                };
                Self::draw_table_row(ui, item, height_cb);
                ui.same_line();
                if ui
                    .selectable_config(&format!("##selectable{i}"))
                    .span_all_columns(true)
                    .selected(selected.is_some_and(|x| x == i))
                    .size([0.0, max_height])
                    .build()
                {
                    selected.replace(i);
                }
                ui.table_next_column();
            }
            tbl.end();
        }
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
