use imgui::Ui;

use super::{object_data_tbl, option_cmp_reverse, templates, Category, Options, SearchItem};
use crate::{err, game_definitions::ItemTemplate, osi_fn, wrappers::osiris};

#[derive(Debug, Clone, Default)]
pub(crate) struct ItemsCategory {
    pub items: Vec<Item>,
    pub options: ItemsOptions,
    pub selected: Option<usize>,
}

impl ItemsCategory {
    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &self.options, &mut self.selected)
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &mut self.items, &mut self.selected);
    }
}

impl Category<2> for ItemsCategory {
    type Item = Item;
    type Options = ItemsOptions;

    const COLS: [&'static str; 2] = ["Display Name", "Internal Name"];

    fn draw_table_row(ui: &Ui, item: &Self::Item, mut height_cb: impl FnMut()) {
        if let Some(display_name) = &item.display_name {
            ui.text_wrapped(display_name);
            height_cb();
        }
        ui.table_next_column();

        ui.text_wrapped(&item.name);
        height_cb();
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        ui.checkbox("Search Internal Name", &mut self.options.search_name)
            || ui.checkbox("Search GUID", &mut self.options.search_id)
            || ui.checkbox("Search Dipsplay Name", &mut self.options.search_display_name)
            || ui.checkbox("Search Description", &mut self.options.search_desc)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Item(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(item: &Self::Item, opts: &Self::Options, pred: impl Fn(&str) -> bool) -> bool {
        opts.search_name && pred(&item.name)
            || opts.search_id && pred(&item.id)
            || opts.search_display_name && item.display_name.as_deref().is_some_and(&pred)
            || opts.search_desc && item.desc.as_deref().is_some_and(&pred)
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        templates()
    }

    fn sort_pred(column: usize) -> fn(&Self::Item, &Self::Item) -> std::cmp::Ordering {
        match column {
            0 => |a, b| option_cmp_reverse(&a.display_name, &b.display_name),
            _ => |a, b| a.name.cmp(&b.name),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ItemsOptions {
    search_name: bool,
    search_id: bool,
    search_display_name: bool,
    search_desc: bool,
}

impl Default for ItemsOptions {
    fn default() -> Self {
        Self { search_name: true, search_id: false, search_display_name: true, search_desc: false }
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
        let id = value.id.into();
        let display_name = (*value.display_name).try_into().ok();
        let desc = (*value.description).try_into().ok();

        Self { name, id, display_name, desc, give_amount: 1 }
    }
}

impl Item {
    pub fn render(&mut self, ui: &Ui) {
        object_data_tbl(ui, |row| {
            row("GUID", &self.id);
            row("Name", &self.name);
            if let Some(display_name) = &self.display_name {
                row("Display Name", display_name);
            }
            if let Some(desc) = &self.desc {
                row("Description", desc);
            }
        });
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

fn give_item(uuid: &str, amount: i32) -> anyhow::Result<()> {
    osi_fn!(TemplateAddTo, uuid, get_host_character()?, amount, 0)?;
    Ok(())
}

fn get_host_character() -> anyhow::Result<osiris::Value> {
    Ok(osi_fn!(GetHostCharacter)?.unwrap())
}
