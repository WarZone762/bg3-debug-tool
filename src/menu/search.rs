use std::borrow::Cow;

use imgui::{TableFlags, Ui};

use self::{
    functions::FunctionCategory,
    spells::SpellCategory,
    table::ObjectTable,
    templates::{GameObjectTemplateCategory, ItemCategory, SceneryCategory},
};
use crate::{game_definitions as gd, globals::Globals};

mod functions;
mod spells;
pub(crate) mod table;
pub(crate) mod table_value;
mod templates;

macro_rules! choose_category {
    ($ident:ident, $($tt:tt)*) => {
        match $ident.cur_category {
            0 => $ident.items.$($tt)*,
            1 => $ident.spells.$($tt)*,
            2 => $ident.functions.$($tt)*,
            3 => $ident.templates.$($tt)*,
            4 => $ident.scenery.$($tt)*,
            _ => unreachable!(),
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
    spells: ObjectTable<SpellCategory>,
    templates: ObjectTable<GameObjectTemplateCategory>,
    scenery: ObjectTable<SceneryCategory>,
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
            spells: ObjectTable::default(),
            templates: ObjectTable::default(),
            scenery: ObjectTable::default(),
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
            &["Items", "Spells", "Osiris Functions", "Templates", "Scenery Templates"],
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

        if let Some(_body) = ui.begin_table_with_flags("body-tbl", 2, TableFlags::RESIZABLE) {
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

        cur_category!(search(&self.text, &self.options));
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
