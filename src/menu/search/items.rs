use imgui::Ui;

use super::{templates2, SearchItem, TableColumn, TableItem, TableItemCategory};
use crate::{err, game_definitions::ItemTemplate, osi_fn, wrappers::osiris};

#[derive(Debug, Clone)]
pub(crate) struct Item {
    name: String,
    id: String,
    display_name: Option<String>,
    desc: Option<String>,
}

impl From<&ItemTemplate> for Item {
    fn from(value: &ItemTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.into();
        let display_name = (*value.display_name).try_into().ok();
        let desc = (*value.description).try_into().ok();

        Self { name, id, display_name, desc }
    }
}

impl TableItem for Item {
    fn columns() -> Box<[super::TableColumn]> {
        Box::new([
            TableColumn::new("Internal Name", true, true),
            TableColumn::new("GUID", false, false),
            TableColumn::new("Display Name", true, true),
            TableColumn::new("Description", false, false),
        ])
    }

    fn draw(&self, ui: &Ui, i: usize) {
        match i {
            0 => super::TableValue::draw(&self.name, ui),
            1 => super::TableValue::draw(&self.id, ui),
            2 => super::TableValue::draw(&self.display_name, ui),
            3 => super::TableValue::draw(&self.desc, ui),
            _ => unreachable!(),
        }
    }

    fn search_str(&self, i: usize) -> String {
        match i {
            0 => super::TableValue::search_str(&self.name),
            1 => super::TableValue::search_str(&self.id),
            2 => super::TableValue::search_str(&self.display_name),
            3 => super::TableValue::search_str(&self.desc),
            _ => unreachable!(),
        }
    }

    fn compare(&self, other: &Self, i: usize) -> std::cmp::Ordering {
        match i {
            0 => super::TableValue::compare(&self.name, &other.name),
            1 => super::TableValue::compare(&self.id, &other.id),
            2 => super::TableValue::compare(&self.display_name, &other.display_name),
            3 => super::TableValue::compare(&self.desc, &other.desc),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ItemCategory {
    give_amount: i32,
}

impl Default for ItemCategory {
    fn default() -> Self {
        Self { give_amount: 1 }
    }
}

impl TableItemCategory for ItemCategory {
    type Item = Item;

    fn source() -> impl Iterator<Item = Self::Item> {
        templates2().filter_map(|x| match x {
            SearchItem::Item(x) => Some(x),
            _ => None,
        })
    }

    fn draw_actions(&mut self, ui: &Ui, item: &mut Self::Item) {
        ui.input_int("Amount", &mut self.give_amount).build();
        if self.give_amount < 1 {
            self.give_amount = 1;
        }

        if ui.button("Give") {
            if let Err(err) = give_item(&item.id, self.give_amount) {
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
