use imgui::Ui;

use super::{object_data_tbl, ObjectField, ObjectTableItem, TableOptions};
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

    signature: String,
    type_string: String,
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

        let signature = if let Some(ret) = &ret_type {
            format!("{name}({}) -> {ret}", args.join(", "))
        } else {
            format!("{name}({})", args.join(", "))
        };

        Self {
            name,
            r#type: f.r#type,
            args,
            ret_type,
            signature,
            type_string: f.r#type.to_string(),
        }
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

impl ObjectTableItem for Function {
    type Options = FunctionOptions;

    fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
        Box::new([
            ObjectField::getter("Signature", true, |x| &x.signature),
            ObjectField::getter("Name", false, |x| &x.name),
            ObjectField::getter("Type", false, |x| &x.type_string),
        ])
    }

    fn source() -> impl Iterator<Item = Self> {
        let fn_db = *Globals::osiris_globals().functions;
        fn_db.as_ref().functions().map(|(k, v)| Function::new(k, v))
    }

    fn filter(&self, opts: &Self::Options) -> bool {
        match self.r#type {
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
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionOptions {
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

impl TableOptions for FunctionOptions {
    fn draw(&mut self, ui: &Ui) -> bool {
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
