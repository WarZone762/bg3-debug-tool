use imgui::Ui;

use super::{object_data_tbl, Category, Options, SearchItem};
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
        Self::draw_table_impl(ui, &mut self.items, &mut self.selected);
    }
}

impl Category<2> for FunctionCategory {
    type Item = Function;
    type Options = FunctionOptions;

    const COLS: [&'static str; 2] = ["Name", "Type"];

    fn draw_table_row(ui: &Ui, item: &Self::Item, mut height_cb: impl FnMut()) {
        if let Some(ret) = &item.ret_type {
            ui.text_wrapped(format!("{}({}) -> {ret}", item.name, item.args.join(", ")));
        } else {
            ui.text_wrapped(format!("{}({})", item.name, item.args.join(", ")));
        }
        height_cb();
        ui.table_next_column();

        ui.text_wrapped(item.r#type.to_string());
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        let mut changed = ui.checkbox("Search Name", &mut self.options.search_name)
            || ui.checkbox("Search Arguments", &mut self.options.search_args);
        if let Some(node) = ui.tree_node("Function Types") {
            changed |= ui.checkbox("Unknown", &mut self.options.incl_unknown);
            changed |= ui.checkbox("Event", &mut self.options.incl_event);
            changed |= ui.checkbox("Query", &mut self.options.incl_query);
            changed |= ui.checkbox("Call", &mut self.options.incl_call);
            changed |= ui.checkbox("Database", &mut self.options.incl_db);
            changed |= ui.checkbox("Procedure", &mut self.options.incl_proc);
            changed |= ui.checkbox("System Query", &mut self.options.incl_sys_query);
            changed |= ui.checkbox("System Call", &mut self.options.incl_sys_call);
            changed |= ui.checkbox("User Query", &mut self.options.incl_user_query);
            node.end();
        }
        changed
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Function(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(item: &Self::Item, opts: &Self::Options, pred: impl Fn(&str) -> bool) -> bool {
        (opts.search_name && pred(&item.name)
            || opts.search_args && item.args.iter().any(|x| pred(x)))
            && match item.r#type {
                game_definitions::FunctionType::Unknown => opts.incl_unknown,
                game_definitions::FunctionType::Event => opts.incl_event,
                game_definitions::FunctionType::Query => opts.incl_query,
                game_definitions::FunctionType::Call => opts.incl_call,
                game_definitions::FunctionType::Database => opts.incl_db,
                game_definitions::FunctionType::Proc => opts.incl_proc,
                game_definitions::FunctionType::SysQuery => opts.incl_sys_query,
                game_definitions::FunctionType::SysCall => opts.incl_sys_call,
                game_definitions::FunctionType::UserQuery => opts.incl_user_query,
            }
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        let fn_db = *Globals::osiris_globals().functions;
        fn_db.as_ref().functions().map(|(k, v)| SearchItem::Function(Function::new(k, v)))
    }

    fn sort_pred(column: usize) -> fn(&Self::Item, &Self::Item) -> std::cmp::Ordering {
        match column {
            0 => |a, b| a.name.cmp(&b.name),
            _ => |a, b| a.r#type.to_string().cmp(&b.r#type.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionOptions {
    search_name: bool,
    search_args: bool,
    incl_unknown: bool,
    incl_event: bool,
    incl_query: bool,
    incl_call: bool,
    incl_db: bool,
    incl_proc: bool,
    incl_sys_query: bool,
    incl_sys_call: bool,
    incl_user_query: bool,
}

impl Default for FunctionOptions {
    fn default() -> Self {
        Self {
            search_name: true,
            search_args: false,
            incl_unknown: true,
            incl_event: true,
            incl_query: true,
            incl_call: true,
            incl_db: true,
            incl_proc: true,
            incl_sys_query: true,
            incl_sys_call: true,
            incl_user_query: true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Function {
    name: String,
    r#type: game_definitions::FunctionType,
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

        Self { name, r#type: f.r#type, args, ret_type }
    }

    pub fn render(&mut self, ui: &Ui) {
        object_data_tbl(ui, |row| {
            row("Type", &self.r#type.to_string());
            row("Name", &self.name);
            for (i, arg) in self.args.iter().enumerate() {
                row(&format!("Argument {i}"), arg);
            }
            if let Some(ret) = &self.ret_type {
                row("Return Type", ret);
            }
        })
    }
}
