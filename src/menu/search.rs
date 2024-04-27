use std::borrow::Cow;

use imgui::{TableFlags, Ui};

use self::{
    functions::FunctionCategory,
    spells::SpellCategory,
    statuses::StatusCategory,
    table::ObjectTable,
    templates::{GameObjectTemplateCategory, ItemCategory, SceneryCategory},
};
use crate::{game_definitions as gd, globals::Globals};

mod functions;
mod osiris_helpers;
mod spells;
mod statuses;
pub(crate) mod table;
pub(crate) mod table_value;
mod templates;

macro_rules! choose_category {
    ($ident:ident, $($tt:tt)*) => {
        match $ident.cur_category {
            0 => $ident.items.$($tt)*,
            1 => $ident.spells.$($tt)*,
            2 => $ident.statuses.$($tt)*,
            3 => $ident.functions.$($tt)*,
            4 => $ident.scenery.$($tt)*,
            5 => $ident.templates.$($tt)*,
            _ => unreachable!(),
        }
    };
}

pub(crate) struct Search {
    reclaim_focus: bool,
    search_failed: bool,
    cur_category: usize,
    text: String,
    options: Options,
    items: ObjectTable<ItemCategory>,
    spells: ObjectTable<SpellCategory>,
    statuses: ObjectTable<StatusCategory>,
    functions: ObjectTable<FunctionCategory>,
    scenery: ObjectTable<SceneryCategory>,
    templates: ObjectTable<GameObjectTemplateCategory>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            reclaim_focus: true,
            search_failed: false,
            cur_category: 0,
            text: String::new(),
            options: Options::default(),
            items: ObjectTable::default(),
            spells: ObjectTable::default(),
            statuses: ObjectTable::default(),
            functions: ObjectTable::default(),
            scenery: ObjectTable::default(),
            templates: ObjectTable::default(),
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
            &["Items", "Spells", "Statuses", "Osiris Functions", "Scenery Templates", "Templates"],
            |x| Cow::from(*x),
        ) && self.text.is_empty()
            && cur_category!(items.len() == 0)
        {
            self.search();
        }

        if let Some(_node) = ui.tree_node("Search Options") {
            if ui.checkbox("Case Sensitive", &mut self.options.case_sensitive) {
                self.search();
            };
            if let Some(_node) = ui.tree_node("Search Fields") {
                if cur_category!(draw_options(ui)) {
                    self.search();
                }
            }
            if let Some(_node) = ui.tree_node("Columns") {
                cur_category!(draw_column_options(ui));
            }
        }
        ui.separator();

        ui.text("Search");

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

        if self.search_failed {
            ui.text("Failed to load items, try loading a save");
        } else if let Some(_body) = ui.begin_table_with_flags("body-tbl", 2, TableFlags::RESIZABLE)
        {
            ui.table_next_row();
            ui.table_set_column_index(0);
            cur_category!(draw_table(ui));
            ui.table_next_column();
            cur_category!(draw_details(ui));
        }
    }

    fn search(&mut self) {
        macro_rules! cur_category {
            ($($tt:tt)*) => {
                choose_category!(self, $($tt)*)
            };
        }

        self.search_failed = cur_category!(search(&self.text, &self.options)).is_none();
        self.reclaim_focus = true;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Options {
    case_sensitive: bool,
}

pub(crate) fn templates() -> impl Iterator<Item = gd::Template<'static>> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| x.value.as_ref().into())
}
