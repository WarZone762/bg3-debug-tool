use imgui::Ui;

use super::{object_data_tbl, option_cmp_reverse, templates, Category, Options, SearchItem};
use crate::game_definitions::{EoCGameObjectTemplate, GameObjectTemplate};

#[derive(Debug, Clone, Default)]
pub(crate) struct OtherCategory {
    pub items: Vec<Other>,
    pub selected: Option<usize>,
    pub options: OtherOptions,
}

impl OtherCategory {
    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &self.options, &mut self.selected);
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &mut self.items, &mut self.selected);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OtherOptions {
    search_name: bool,
    search_id: bool,
    search_display_name: bool,
}

impl Default for OtherOptions {
    fn default() -> Self {
        Self { search_name: true, search_id: false, search_display_name: true }
    }
}

impl Category<3> for OtherCategory {
    type Item = Other;
    type Options = OtherOptions;

    const COLS: [&'static str; 3] = ["Display Name", "Internal Name", "Type"];

    fn draw_table_row(ui: &Ui, item: &Self::Item, mut height_cb: impl FnMut()) {
        if let Some(display_name) = &item.display_name {
            ui.text_wrapped(display_name);
            height_cb();
        }
        ui.table_next_column();

        ui.text_wrapped(&item.name);
        height_cb();
        ui.table_next_column();

        ui.text_wrapped(&item.r#type);
        height_cb();
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        ui.checkbox("Search Internal Name", &mut self.options.search_name)
            || ui.checkbox("Search GUID", &mut self.options.search_id)
            || ui.checkbox("Search Display Name", &mut self.options.search_display_name)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Other(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(item: &Self::Item, opts: &Self::Options, pred: impl Fn(&str) -> bool) -> bool {
        opts.search_name && pred(&item.name)
            || opts.search_id && pred(&item.id)
            || opts.search_display_name && item.display_name.as_deref().is_some_and(pred)
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        templates()
    }

    fn sort_pred(column: usize) -> fn(&Self::Item, &Self::Item) -> std::cmp::Ordering {
        match column {
            0 => |a, b| option_cmp_reverse(&a.display_name, &b.display_name),
            1 => |a, b| a.name.cmp(&b.name),
            _ => |a, b| a.r#type.cmp(&b.r#type),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Other {
    name: String,
    id: String,
    display_name: Option<String>,
    r#type: String,
}

impl From<&EoCGameObjectTemplate> for Other {
    fn from(value: &EoCGameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.into();
        let display_name = (*value.display_name).try_into().ok();

        Self { name, id, display_name, r#type: value.get_type().into() }
    }
}

impl From<&GameObjectTemplate> for Other {
    fn from(value: &GameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.into();

        Self { name, id, display_name: None, r#type: value.get_type().into() }
    }
}

impl Other {
    pub fn render(&mut self, ui: &Ui) {
        object_data_tbl(ui, |row| {
            row("Type", &self.r#type);
            row("GUID", &self.id);
            row("Name", &self.name);
            if let Some(display_name) = &self.display_name {
                row("Display Name", display_name);
            }
        })
    }
}
