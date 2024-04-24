use imgui::Ui;

use super::{TableColumn, TableItem, TableItemCategory};
use crate::{
    game_definitions::{self, OsiStr, ValueType},
    globals::Globals,
};

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
}

impl TableItem for Function {
    fn columns() -> Box<[TableColumn]> {
        Box::new([TableColumn::new("Signature", true, true), TableColumn::new("Type", true, true)])
    }

    fn draw(&self, ui: &Ui, i: usize) {
        match i {
            0 => ui.text_wrapped(if let Some(ret) = &self.ret_type {
                format!("{}({}) -> {ret}", self.name, self.args.join(", "))
            } else {
                format!("{}({})", self.name, self.args.join(", "))
            }),
            1 => ui.text_wrapped(self.r#type.to_string()),
            _ => unreachable!(),
        }
    }

    fn search_str(&self, i: usize) -> String {
        match i {
            0 => self.name.clone(),
            1 => self.r#type.to_string(),
            _ => unreachable!(),
        }
    }

    fn compare(&self, other: &Self, i: usize) -> std::cmp::Ordering {
        self.search_str(i).cmp(&other.search_str(i))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FunctionCategory {
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

impl Default for FunctionCategory {
    fn default() -> Self {
        Self {
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

impl TableItemCategory for FunctionCategory {
    type Item = Function;

    fn source() -> impl Iterator<Item = Self::Item> {
        let fn_db = *Globals::osiris_globals().functions;
        fn_db.as_ref().functions().map(|(k, v)| Function::new(k, v))
    }

    fn filter(&self, item: &Self::Item) -> bool {
        match item.r#type {
            game_definitions::FunctionType::Unknown => self.incl_unknown,
            game_definitions::FunctionType::Event => self.incl_event,
            game_definitions::FunctionType::Query => self.incl_query,
            game_definitions::FunctionType::Call => self.incl_call,
            game_definitions::FunctionType::Database => self.incl_db,
            game_definitions::FunctionType::Proc => self.incl_proc,
            game_definitions::FunctionType::SysQuery => self.incl_sys_query,
            game_definitions::FunctionType::SysCall => self.incl_sys_call,
            game_definitions::FunctionType::UserQuery => self.incl_user_query,
        }
    }

    fn draw_options(&mut self, ui: &Ui) -> bool {
        let mut changed = false;
        if let Some(node) = ui.tree_node("Function Types") {
            changed |= ui.checkbox("Unknown", &mut self.incl_unknown);
            changed |= ui.checkbox("Event", &mut self.incl_event);
            changed |= ui.checkbox("Query", &mut self.incl_query);
            changed |= ui.checkbox("Call", &mut self.incl_call);
            changed |= ui.checkbox("Database", &mut self.incl_db);
            changed |= ui.checkbox("Procedure", &mut self.incl_proc);
            changed |= ui.checkbox("System Query", &mut self.incl_sys_query);
            changed |= ui.checkbox("System Call", &mut self.incl_sys_call);
            changed |= ui.checkbox("User Query", &mut self.incl_user_query);
            node.end();
        }
        changed
    }
}
