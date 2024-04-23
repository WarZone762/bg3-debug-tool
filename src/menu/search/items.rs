use imgui::Ui;

use super::{object_data_tbl, templates, ObjectField, ObjectTableItem, SearchItem};
use crate::{err, game_definitions::ItemTemplate, osi_fn, wrappers::osiris};

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

impl ObjectTableItem for Item {
    type Options = ();

    fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
        Box::new([
            ObjectField::getter("Internal Name", true, |x| &x.name),
            ObjectField::getter("GUID", false, |x| &x.id),
            ObjectField::getter("Display Name", true, |x| &x.display_name),
            ObjectField::getter("Description", false, |x| &x.desc),
        ])
    }

    fn source() -> impl Iterator<Item = Self> {
        templates().filter_map(|x| match x {
            SearchItem::Item(x) => Some(x),
            _ => None,
        })
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
