use std::borrow::Cow;

use imgui::{MouseButton, TableFlags, Ui};

use crate::{
    err,
    game_definitions::{
        self, EoCGameObjectTemplate, GameObjectTemplate, ItemTemplate, OsiStr, SpellPrototype,
        Template, ValueType,
    },
    globals::Globals,
    info, osi_fn,
    wrappers::osiris,
};

macro_rules! choose_category {
    ($ident:ident, $($tt:tt)*) => {
        match $ident.cur_category {
            0 => $ident.items.$($tt)*,
            1 => $ident.spells.$($tt)*,
            2 => $ident.functions.$($tt)*,
            3 => $ident.other.$($tt)*,
            _ => $ident.other.$($tt)*,
        }
    };
}

#[derive(Debug, Clone)]
pub(crate) struct Search {
    cur_category: usize,
    text: String,
    options: Options,
    items: ItemsCategory,
    spells: SpellCategory,
    functions: FunctionCategory,
    other: OtherCategory,
}

impl Search {
    pub fn new() -> Self {
        Self {
            cur_category: 0,
            text: String::new(),
            options: Options::new(),
            items: ItemsCategory::new(),
            spells: SpellCategory::new(),
            functions: FunctionCategory::default(),
            other: OtherCategory::new(),
        }
    }

    pub fn render(&mut self, ui: &Ui) {
        macro_rules! cur_category {
            ($($tt:tt)*) => {
                choose_category!(self, $($tt)*)
            };
        }

        if let Some(node) = ui.tree_node("Search Options") {
            if ui.checkbox("Case Sensitive", &mut self.options.case_sensitive) {
                self.search();
            };
            if let Some(node) = ui.tree_node("Search Fields") {
                if ui.checkbox("Search Name", &mut self.options.search_name)
                    || ui.checkbox("Search GUID", &mut self.options.search_id)
                    || ui.checkbox("Search Display Name", &mut self.options.search_display_name)
                    || cur_category!(draw_options(ui))
                {
                    self.search();
                }
                node.pop();
            }
            node.pop();
        }
        ui.combo(
            "Object Category",
            &mut self.cur_category,
            &["Items", "Spells", "Osiris Functions", "Other"],
            |x| Cow::from(*x),
        );
        ui.separator();
        ui.text("Search");
        if ui.input_text("<<", &mut self.text).enter_returns_true(true).build() {
            self.search();
        }

        ui.text(format!("found {} entries", cur_category!(items.len())));

        cur_category!(draw_table(ui));

        ui.same_line();
        ui.child_window("Object Data").build(|| {
            if let Some(selected_item) = cur_category!(selected) {
                cur_category!(items[selected_item].render(ui));
            }
        });
    }

    fn search(&mut self) {
        macro_rules! cur_category {
            ($($tt:tt)*) => {
                choose_category!(self, $($tt)*)
            };
        }

        cur_category!(search(&self.text, &self.options));
    }
}

#[derive(Debug, Clone, Copy)]
struct Options {
    case_sensitive: bool,
    search_name: bool,
    search_id: bool,
    search_display_name: bool,
}

impl Options {
    pub fn new() -> Self {
        Self {
            case_sensitive: false,
            search_name: true,
            search_id: false,
            search_display_name: true,
        }
    }
}

#[derive(Debug, Clone)]
struct ItemsCategory {
    items: Vec<Item>,
    options: ItemsOptions,
    selected: Option<usize>,
}

impl ItemsCategory {
    pub fn new() -> Self {
        Self { items: Vec::new(), options: ItemsOptions::new(), selected: None }
    }

    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &self.options, &mut self.selected)
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &self.items, &mut self.selected);
    }
}

impl Category for ItemsCategory {
    type Item = Item;
    type Options = ItemsOptions;

    const COLS: usize = 2;

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
        ui.checkbox("Search Description", &mut self.options.search_desc)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Item(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool {
        opts.search_name && pred(&item.name)
            || opts.search_id && pred(&item.id)
            || opts.search_display_name && item.display_name.as_deref().is_some_and(&pred)
            || self_opts.search_desc && item.desc.as_deref().is_some_and(&pred)
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        templates()
    }
}

#[derive(Debug, Clone, Copy)]
struct ItemsOptions {
    search_desc: bool,
}

impl ItemsOptions {
    pub fn new() -> Self {
        Self { search_desc: false }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SpellCategory {
    items: Vec<Spell>,
    selected: Option<usize>,
    options: SpellOptions,
}

impl SpellCategory {
    pub fn new() -> Self {
        Self { items: Vec::new(), selected: None, options: SpellOptions::new() }
    }

    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &self.options, &mut self.selected)
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &self.items, &mut self.selected);
    }
}

impl Category for SpellCategory {
    type Item = Spell;
    type Options = SpellOptions;

    const COLS: usize = 2;

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
        ui.checkbox("Search Description", &mut self.options.search_desc)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Spell(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool {
        opts.search_display_name && item.display_name.as_deref().is_some_and(&pred)
            || self_opts.search_desc && item.desc.as_deref().is_some_and(&pred)
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        spells()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SpellOptions {
    search_desc: bool,
}

impl SpellOptions {
    pub fn new() -> Self {
        Self { search_desc: false }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FunctionCategory {
    items: Vec<Function>,
    selected: Option<usize>,
    options: FunctionOptions,
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
        ui.checkbox("Search Arguments", &mut self.options.search_args)
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Function(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool {
        opts.search_display_name && pred(&item.name)
            || self_opts.search_args && item.args.iter().any(|x| pred(x))
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        functions()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FunctionOptions {
    search_args: bool,
}

#[derive(Debug, Clone)]
struct OtherCategory {
    items: Vec<Other>,
    selected: Option<usize>,
}

impl OtherCategory {
    pub fn new() -> Self {
        Self { items: Vec::new(), selected: None }
    }

    pub fn search(&mut self, text: &str, opts: &Options) {
        Self::search_impl(&mut self.items, text, opts, &(), &mut self.selected);
    }

    pub fn draw_table(&mut self, ui: &Ui) {
        Self::draw_table_impl(ui, &self.items, &mut self.selected);
    }
}

impl Category for OtherCategory {
    type Item = Other;
    type Options = ();

    const COLS: usize = 2;

    fn draw_table_row(ui: &Ui, item: &Self::Item, mut height_cb: impl FnMut()) {
        if let Some(display_name) = &item.display_name {
            ui.text_wrapped(display_name);
            height_cb();
        }
        ui.table_next_column();

        ui.text_wrapped(&item.name);
        height_cb();
    }

    fn draw_options(&mut self, _ui: &Ui) -> bool {
        false
    }

    fn search_filter_map(item: SearchItem) -> Option<Self::Item> {
        match item {
            SearchItem::Other(x) => Some(x),
            _ => None,
        }
    }

    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        _self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool {
        opts.search_name && pred(&item.name)
            || opts.search_id && pred(&item.id)
            || opts.search_display_name && item.display_name.as_deref().is_some_and(pred)
    }

    fn search_iter() -> impl Iterator<Item = SearchItem> {
        templates()
    }
}

trait Category {
    const COLS: usize;
    type Item;
    type Options;

    fn draw_table_row(ui: &Ui, item: &Self::Item, height_cb: impl FnMut());
    fn draw_options(&mut self, ui: &Ui) -> bool;
    fn search_filter_map(item: SearchItem) -> Option<Self::Item>;
    fn search_filter(
        item: &Self::Item,
        opts: &Options,
        self_opts: &Self::Options,
        pred: impl Fn(&str) -> bool,
    ) -> bool;

    fn search_iter() -> impl Iterator<Item = SearchItem>;

    fn search_impl(
        items: &mut Vec<Self::Item>,
        text: &str,
        opts: &Options,
        self_opts: &Self::Options,
        selected: &mut Option<usize>,
    ) {
        selected.take();
        items.clear();
        if opts.case_sensitive {
            let pred = &|string: &str| string.contains(text);
            items.extend(
                Self::search_iter()
                    .filter_map(Self::search_filter_map)
                    .filter(|x| Self::search_filter(x, opts, self_opts, pred)),
            )
        } else {
            let text = text.to_lowercase();
            let pred = &|string: &str| string.to_lowercase().contains(&text);
            items.extend(
                Self::search_iter()
                    .filter_map(Self::search_filter_map)
                    .filter(|x| Self::search_filter(x, opts, self_opts, pred)),
            );
        }
    }

    fn draw_table_impl(ui: &Ui, items: &[Self::Item], selected: &mut Option<usize>) {
        let size = ui.window_size();
        if let Some(tbl) = ui.begin_table_with_sizing(
            "items-tbl",
            Self::COLS,
            TableFlags::SCROLL_Y,
            [size[0] * 0.5, -1.0],
            0.0,
        ) {
            ui.table_next_row();
            for (i, item) in items.iter().enumerate() {
                ui.table_set_column_index(0);

                let mut max_height = 0.0;
                let height_cb = || {
                    max_height = ui.item_rect_size()[1].max(max_height);
                };
                Self::draw_table_row(ui, item, height_cb);
                ui.same_line();
                if ui
                    .selectable_config(&format!("##selectable{i}"))
                    .span_all_columns(true)
                    .selected(selected.is_some_and(|x| x == i))
                    .size([0.0, max_height])
                    .build()
                {
                    selected.replace(i);
                }
                ui.table_next_column();
            }
            tbl.end();
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum SearchItem {
    // CombinedLight
    // LevelTemplate
    // Schematic
    // Spline
    // TileConstruction
    // character
    // constellation
    // constellationHelper
    // decal
    // fogVolume
    // item
    // light
    // lightProbe
    // prefab
    // projectile
    // scenery
    // surface
    // terrain
    // trigger
    Item(Item),
    Other(Other),
    Spell(Spell),
    Function(Function),
}

impl From<Template<'_>> for SearchItem {
    fn from(value: Template) -> Self {
        match value {
            Template::GameObject(x) => Self::Other(x.into()),
            Template::EoCGameObject(x) => Self::Other(x.into()),
            Template::Scenery(x) => Self::Other((&x.base).into()),
            Template::Item(x) => Self::Item(x.into()),
        }
    }
}

impl From<&SpellPrototype> for SearchItem {
    fn from(value: &SpellPrototype) -> Self {
        Self::Spell(value.into())
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
        let id = value.id.try_into().unwrap();
        let display_name = (*value.display_name).try_into().ok();
        let desc = (*value.description).try_into().ok();

        Self { name, id, display_name, desc, give_amount: 1 }
    }
}

impl Item {
    pub fn render(&mut self, ui: &Ui) {
        if let Some(tbl) = ui.begin_table("obj-data-tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            object_data_row(ui, "GUID", &self.id);
            object_data_row(ui, "Name", &self.name);
            if let Some(display_name) = &self.display_name {
                object_data_row(ui, "Display Name", display_name);
            }
            if let Some(desc) = &self.desc {
                object_data_row(ui, "Description", desc);
            }

            tbl.end();

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
}

#[derive(Debug, Clone)]
pub(crate) struct Other {
    name: String,
    id: String,
    display_name: Option<String>,
}

impl From<&EoCGameObjectTemplate> for Other {
    fn from(value: &EoCGameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.try_into().unwrap();
        let display_name = (*value.display_name).try_into().ok();

        Self { name, id, display_name }
    }
}

impl From<&GameObjectTemplate> for Other {
    fn from(value: &GameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.try_into().unwrap();

        Self { name, id, display_name: None }
    }
}

impl Other {
    pub fn render(&mut self, ui: &Ui) {
        if let Some(tbl) = ui.begin_table("obj-data-tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            object_data_row(ui, "GUID", &self.id);
            object_data_row(ui, "Name", &self.name);
            if let Some(display_name) = &self.display_name {
                object_data_row(ui, "Display Name", display_name);
            }

            tbl.end();
        }
    }
}

#[derive(Debug, Clone)]
struct Spell {
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
        if let Some(tbl) = ui.begin_table("obj-data-tbl", 2) {
            ui.table_next_row();
            ui.table_set_column_index(0);

            if let Some(display_name) = &self.display_name {
                object_data_row(ui, "Display Name", display_name);
            }
            if let Some(desc) = &self.display_name {
                object_data_row(ui, "Description", desc);
            }

            tbl.end();
        }
    }
}

#[derive(Debug, Clone)]
struct Function {
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

fn object_data_row(ui: &Ui, name: &str, text: &str) {
    ui.text(name);
    ui.table_next_column();
    ui.text_wrapped(text);
    copy_popup(ui, text);
    ui.table_next_column();
}

fn copy_popup(ui: &Ui, copy_text: &str) {
    if ui.is_item_hovered() {
        if ui.is_mouse_clicked(MouseButton::Right) {
            ui.set_clipboard_text(copy_text);
        }
        if ui.clipboard_text().is_some_and(|x| x == copy_text) {
            ui.tooltip(|| ui.text("Copied!"));
        } else {
            ui.tooltip(|| ui.text("Right click to copy"));
        }
    }
}

fn templates() -> impl Iterator<Item = SearchItem> {
    let template_manager = *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
    let template_bank = template_manager.global_template_bank();

    template_bank.templates.iter().map(|x| Template::from(x.value.as_ref()).into())
}

fn spells() -> impl Iterator<Item = SearchItem> {
    let spell_manager = *Globals::static_symbols().eoc__SpellPrototypeManager.unwrap();
    spell_manager.as_ref().spells.iter().map(|x| x.as_ref().into())
}

fn functions() -> impl Iterator<Item = SearchItem> {
    let fn_db = *Globals::osiris_globals().functions;
    fn_db.as_ref().functions().map(|(k, v)| SearchItem::Function(Function::new(k, v)))
}

fn give_item(uuid: &str, amount: i32) -> anyhow::Result<()> {
    osi_fn!(TemplateAddTo, uuid, get_host_character()?, amount, 0)?;
    Ok(())
}

fn get_host_character() -> anyhow::Result<osiris::Value> {
    Ok(osi_fn!(GetHostCharacter)?.unwrap())
}
