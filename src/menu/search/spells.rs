use imgui::Ui;

use super::{object_data_tbl, option_cmp_reverse, Category, Options, SearchItem};
use crate::{game_definitions::SpellPrototype, globals::Globals};

#[derive(Debug, Clone, Default)]
pub(crate) struct SpellCategory {
    pub items: Vec<Spell>,
    pub selected: Option<usize>,
    pub options: SpellOptions,
}

impl SpellCategory {
    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &self.options, &mut self.selected)
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &mut self.items, &mut self.selected);
    }
}

impl Category<2> for SpellCategory {
    type Item = Spell;
    type Options = SpellOptions;

    const COLS: [&'static str; 2] = ["Display Name", "Description"];

    fn draw_table_row(ui: &Ui, item: &Self::Item, mut height_cb: impl FnMut()) {
        if let Some(display_name) = &item.display_name {
            ui.text_wrapped(display_name);
            height_cb();
        }
        ui.table_next_column();

        if let Some(desc) = &item.desc {
            ui.text_wrapped(desc);
            height_cb();
        }
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        ui.checkbox("Search Display Name", &mut self.options.search_display_name)
            || ui.checkbox("Search Description", &mut self.options.search_desc)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Spell(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(item: &Self::Item, opts: &Self::Options, pred: impl Fn(&str) -> bool) -> bool {
        opts.search_display_name && item.display_name.as_deref().is_some_and(&pred)
            || opts.search_desc && item.desc.as_deref().is_some_and(&pred)
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        let spell_manager = *Globals::static_symbols().eoc__SpellPrototypeManager.unwrap();
        spell_manager.as_ref().spells.iter().map(|x| x.as_ref().into())
    }

    fn sort_pred(column: usize) -> fn(&Self::Item, &Self::Item) -> std::cmp::Ordering {
        match column {
            0 => |a, b| option_cmp_reverse(&a.display_name, &b.display_name),
            _ => |a, b| option_cmp_reverse(&a.desc, &b.desc),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SpellOptions {
    search_display_name: bool,
    search_desc: bool,
}

impl Default for SpellOptions {
    fn default() -> Self {
        Self { search_display_name: true, search_desc: false }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Spell {
    display_name: Option<String>,
    desc: Option<String>,
}

impl From<&SpellPrototype> for Spell {
    fn from(value: &SpellPrototype) -> Self {
        let display_name = value.description.display_name.try_into().ok();
        let desc = value.description.description.try_into().ok();

        Self { display_name, desc }
    }
}

impl Spell {
    pub fn render(&mut self, ui: &Ui) {
        object_data_tbl(ui, |row| {
            if let Some(display_name) = &self.display_name {
                row("Display Name", display_name);
            }
            if let Some(desc) = &self.display_name {
                row("Description", desc);
            }
        })
    }
}
