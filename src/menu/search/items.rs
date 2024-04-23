use imgui::Ui;

use super::{templates, ObjectField, ObjectTableItem, SearchItem, TableItemActions};
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

impl ObjectTableItem for Item {
    type ActionMenu = ItemMenu;
    type Options = ();

    fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
        // let g = for<'a> |x: &'a Self| -> &'a String { &x.name };
        Box::new([
            ObjectField::define("Internal Name", true, for<'a> |x: &'a Self| -> &'a str {
                &x.name
            }),
            ObjectField::define("GUID", false, for<'a> |x: &'a Self| -> &'a str { &x.id }),
            ObjectField::define("Display Name", true, for<'a> |x: &'a Self| -> Option<&'a str> {
                x.display_name.as_deref()
            }),
            ObjectField::define("Description", false, for<'a> |x: &'a Self| -> Option<&'a str> {
                x.desc.as_deref()
            }),
        ])
    }

    fn source() -> impl Iterator<Item = Self> {
        templates().filter_map(|x| match x {
            SearchItem::Item(x) => Some(x),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ItemMenu {
    give_amount: i32,
}

impl Default for ItemMenu {
    fn default() -> Self {
        Self { give_amount: 1 }
    }
}

impl TableItemActions<Item> for ItemMenu {
    fn draw(&mut self, ui: &Ui, item: &mut Item) {
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
