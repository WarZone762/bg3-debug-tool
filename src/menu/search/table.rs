use std::cmp::Ordering;

use imgui::{
    MouseButton, TableColumnFlags, TableColumnSetup, TableFlags, TableSortDirection, TableToken, Ui,
};
use itertools::Itertools;

use super::{
    table_value::{
        GameObjectFullVisitor, GameObjectParallelVisitor, GameObjectVisitor, TableValue,
    },
    Options,
};

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
    pub fn search(&mut self, string: &str, opts: &Options) -> Option<()> {
        self.selected.take();
        self.items.clear();
        let mut search = |string: &str, pred: fn(&str, &str) -> bool| -> Option<()> {
            self.items.extend(T::source()?.filter(|x| {
                self.columns.iter().enumerate().filter(|(_, col)| col.included_in_search).any(
                    |(i, _)| {
                        pred(x.visit(&mut SearchVisitor, i).as_ref(), string)
                            && self.category.filter(x)
                    },
                )
            }));
            Some(())
        };

        if opts.case_sensitive {
            search(string, |text, string| text.contains(string))
        } else {
            let string = string.to_lowercase();
            search(&string, |text, string| text.to_lowercase().contains(string))
        }
    }

    pub fn draw_table(&mut self, ui: &Ui) {
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

        let visible_cols = self.columns.iter().filter(|x| x.visible).collect_vec();
        if let Some(tbl) = ui.begin_table_with_sizing(
            "items-tbl",
            visible_cols.len(),
            TableFlags::SCROLL_Y
                | TableFlags::RESIZABLE
                | TableFlags::REORDERABLE
                | TableFlags::SORTABLE,
            [0.0, -1.0],
            0.0,
        ) {
            ui.table_setup_scroll_freeze(0, 1);
            for field in &visible_cols {
                ui.table_setup_column_with(imgui::TableColumnSetup {
                    name: field.name.as_str(),
                    flags: TableColumnFlags::default(),
                    ..Default::default()
                });
            }
            ui.table_headers_row();
            ui.table_next_row();
            if let Some(specs) = ui.table_sort_specs_mut() {
                specs.conditional_sort(|specs| {
                    if let Some(specs) = specs.iter().next() {
                        match specs.sort_direction() {
                            Some(TableSortDirection::Ascending) => self.items.sort_by(|a, b| {
                                a.visit_parallel(&mut Comparator, b, specs.column_idx())
                            }),
                            Some(TableSortDirection::Descending) => self.items.sort_by(|a, b| {
                                a.visit_parallel(&mut Comparator, b, specs.column_idx()).reverse()
                            }),
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
                let mut columns = self.columns.iter().enumerate().filter(|(_, x)| x.visible);
                ui.table_set_column_index(0);
                let mut max_height = item
                    .visit(&mut RowDrawer::new(ui, i), columns.next().map(|(i, _)| i).unwrap_or(0));
                for (j, _) in columns {
                    ui.table_next_column();
                    max_height = max_height.max(item.visit(&mut RowDrawer::new(ui, i), j));
                }
                ui.same_line();

                for j in 0..visible_cols.len() {
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

    pub fn draw_options(&mut self, ui: &Ui) -> bool {
        let mut changed = false;
        for col in self.columns.iter_mut() {
            changed |= ui.checkbox(&col.name, &mut col.included_in_search);
        }
        changed || self.category.draw_options(ui)
    }

    pub fn draw_column_options(&mut self, ui: &Ui) -> bool {
        let mut changed = false;
        for col in self.columns.iter_mut() {
            changed |= ui.checkbox(&col.name, &mut col.visible);
        }
        changed
    }

    pub fn draw_details(&mut self, ui: &Ui) {
        if let Some(selected) = self.selected {
            let item = &mut self.items[selected];

            if let Some(_tbl) = details_table(ui) {
                details_view(ui, item);
            }
            self.category.draw_actions(ui, item);
        }
    }
}

#[must_use]
pub(crate) fn details_table(ui: &Ui) -> Option<TableToken> {
    if let Some(tbl) = ui.begin_table_with_sizing(
        "obj-details-tbl",
        2,
        TableFlags::RESIZABLE | TableFlags::SIZING_STRETCH_SAME,
        [0.0, 0.0],
        0.0,
    ) {
        for col in [
            TableColumnSetup { name: "Field", init_width_or_weight: 0.4, ..Default::default() },
            TableColumnSetup {
                name: "Value",
                init_width_or_weight: 0.6,
                flags: TableColumnFlags::NO_CLIP,
                ..Default::default()
            },
        ] {
            ui.table_setup_column_with(col)
        }
        ui.table_next_row();
        ui.table_set_column_index(0);
        return Some(tbl);
    }
    None
}

pub(crate) fn details_view(ui: &Ui, item: &impl TableItem) {
    item.visit_all(DetailsDrawer(ui));
}

#[derive(Debug)]
pub(crate) struct DetailsDrawer<'a>(pub &'a Ui);
impl GameObjectVisitor for DetailsDrawer<'_> {
    type Return = ();

    fn visit(&mut self, name: impl AsRef<str>, item: &impl TableValue) {
        if !item.is_defined() {
            return;
        }
        let ui = self.0;
        let name = name.as_ref();
        if item.is_container() {
            let node = ui.tree_node(format!("{name}: {}", type_name(item)));
            copy_tooltip(ui, item.export_str());
            ui.table_next_column();
            ui.table_next_column();
            if let Some(_node) = node {
                let id = ui.push_id(name);
                item.draw(ui);
                id.pop();
                ui.table_next_row();
                ui.table_set_column_index(0);
            }
            return;
        }

        ui.text_wrapped(format!("{name}: {}", type_name(item)));
        ui.table_next_column();
        let id = ui.push_id(name);
        item.draw(ui);
        id.pop();
        copy_tooltip(ui, item.export_str());

        ui.table_next_column();

        fn type_name<T: TableValue>(_item: &T) -> String {
            T::type_name()
        }
    }
}

pub(crate) fn copy_tooltip(ui: &Ui, text: impl AsRef<str>) {
    if ui.is_item_hovered() {
        let text = text.as_ref();
        if ui.is_mouse_clicked(MouseButton::Right) {
            ui.set_clipboard_text(text);
        }
        if ui.clipboard_text().is_some_and(|x| x == text) {
            ui.tooltip(|| ui.text("Copied!"));
        } else {
            ui.tooltip(|| ui.text("Right click to copy"));
        }
    }
}

impl GameObjectFullVisitor for DetailsDrawer<'_> {
    type Finish = ();

    fn finish(self) -> Self::Finish {}
}

#[derive(Debug)]
pub(crate) struct RowDrawer<'a>(&'a Ui, usize);

impl<'a> RowDrawer<'a> {
    pub fn new(ui: &'a Ui, index: usize) -> Self {
        Self(ui, index)
    }
}

impl GameObjectVisitor for RowDrawer<'_> {
    type Return = f32;

    fn visit(&mut self, name: impl AsRef<str>, item: &impl TableValue) -> Self::Return {
        let ui = self.0;

        let name = name.as_ref();
        if item.is_container() {
            let node_id = ui.push_id_usize(self.1);
            if let Some(_node) = ui.tree_node(name) {
                if let Some(_tbl) = details_table(ui) {
                    let id = ui.push_id(name);
                    item.draw(ui);
                    id.pop();
                }
            }
            node_id.pop();
        } else {
            let id = ui.push_id_usize(self.1);
            item.draw(ui);
            id.pop();
        }
        ui.item_rect_size()[1]
    }
}

#[derive(Debug)]
pub(crate) struct Comparator;

impl GameObjectParallelVisitor for Comparator {
    type Return = Ordering;

    fn visit_parallel<T: TableValue>(
        &mut self,
        _name: impl AsRef<str>,
        a: &T,
        b: &T,
    ) -> Self::Return {
        a.tbl_cmp(b)
    }
}

#[derive(Debug)]
pub(crate) struct SearchVisitor;

impl GameObjectVisitor for SearchVisitor {
    type Return = String;

    fn visit(&mut self, _name: impl AsRef<str>, item: &impl TableValue) -> Self::Return {
        let fmt = std::fmt::FormatterFn(|f| item.search_str(f));
        format!("{fmt}")
    }
}

pub(crate) trait TableItemCategory: Default {
    type Item: ColumnsTableItem;

    fn source() -> Option<impl Iterator<Item = Self::Item>>;
    fn filter(&self, _item: &Self::Item) -> bool {
        true
    }
    fn draw_options(&mut self, _ui: &Ui) -> bool {
        false
    }
    fn draw_actions(&mut self, _ui: &Ui, _item: &mut Self::Item) {}
}

#[derive(Debug, Clone)]
pub(crate) struct TableColumn {
    name: String,
    visible: bool,
    included_in_search: bool,
}

impl TableColumn {
    pub fn new(name: impl AsRef<str>, visible: bool, included_in_search: bool) -> Self {
        Self { name: name.as_ref().to_string(), included_in_search, visible }
    }
}

pub(crate) trait ColumnsTableItem: TableItem {
    fn columns() -> Box<[TableColumn]>;
    fn visit_parallel<T: GameObjectParallelVisitor>(
        &self,
        visitor: &mut T,
        other: &Self,
        i: usize,
    ) -> T::Return;
}

pub(crate) trait TableItem {
    fn visit<T: GameObjectVisitor>(&self, visitor: &mut T, i: usize) -> T::Return;
    fn visit_field<T: GameObjectVisitor>(&self, visitor: &mut T, name: &str) -> Option<T::Return>;
    fn visit_all<T: GameObjectFullVisitor>(&self, visitor: T) -> T::Finish;
}
