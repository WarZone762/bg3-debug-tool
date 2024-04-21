use imgui::Ui;

use super::{object_data_row, Category, Options, SearchItem};
use crate::{
    game_definitions::{self, OsiStr, ValueType},
    globals::Globals,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct FunctionCategory {
    pub items: Vec<Function>,
    pub selected: Option<usize>,
    pub options: FunctionOptions,
}

impl FunctionCategory {
    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &self.options, &mut self.selected);
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &self.items, &mut self.selected);
    }
}

impl Category for FunctionCategory {
    type Item = Function;
    type Options = FunctionOptions;

    const COLS: usize = 1;

    fn draw_table_row(ui: &Ui, item: &Self::Item, mut height_cb: impl FnMut()) {
        if let Some(ret) = &item.ret_type {
            ui.text_wrapped(format!("{}({}) -> {ret}", item.name, item.args.join(", ")));
        } else {
            ui.text_wrapped(format!("{}({})", item.name, item.args.join(", ")));
        }
        height_cb();
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        ui.checkbox("Search Name", &mut self.options.search_name)
            || ui.checkbox("Search Arguments", &mut self.options.search_args)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Function(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(item: &Self::Item, opts: &Self::Options, pred: impl Fn(&str) -> bool) -> bool {
        opts.search_name && pred(&item.name)
            || opts.search_args && item.args.iter().any(|x| pred(x))
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        functions()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionOptions {
    search_name: bool,
    search_args: bool,
}

impl Default for FunctionOptions {
    fn default() -> Self {
        Self { search_name: true, search_args: false }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Function {
    name: String,
    args: Vec<String>,
    ret_type: Option<String>,
}

impl Function {
    pub fn new(name: &OsiStr, f: &game_definitions::Function) -> Self {
        let name = name.to_string().rsplit_once('/').unwrap().0.into();
        let mut args = Vec::with_capacity(f.signature.params.params.size as _);
        let mut ret_type = None;
        for (i, arg) in f.signature.params.params.iter().enumerate() {
            if f.signature.out_param_list.is_out_param(i) {
                if ret_type.is_none() {
                    ret_type = Some(format!("{:?}", ValueType::from(arg.r#type)))
                } else {
                    args.push(format!("OUT {:?}", ValueType::from(arg.r#type)));
                }
            } else {
                args.push(format!("{:?}", ValueType::from(arg.r#type)));
            }
        }

        Self { name, args, ret_type }
    }

    pub fn render(&mut self, ui: &Ui) {
        if let Some(tbl) = ui.begin_table("obj-data-tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            object_data_row(ui, "Name", &self.name);
            for (i, arg) in self.args.iter().enumerate() {
                object_data_row(ui, &format!("Argument {i}"), arg);
            }
            if let Some(ret) = &self.ret_type {
                object_data_row(ui, "Return Type", ret);
            }

            tbl.end();
        }
    }
}

fn functions() -> impl Iterator<Item = SearchItem> {
    let fn_db = *Globals::osiris_globals().functions;
    fn_db.as_ref().functions().map(|(k, v)| SearchItem::Function(Function::new(k, v)))
}
