use game_object::GameObject;
use imgui::Ui;

use super::{osiris_helpers::give_item, table::TableItemCategory, templates};
use crate::{
    err,
    game_definitions::{self as gd, GameObjectTemplate, ItemTemplate, SceneryTemplate},
};

#[derive(Default)]
pub(crate) struct GameObjectTemplateCategory;
impl TableItemCategory for GameObjectTemplateCategory {
    type Item = &'static GameObjectTemplate;

    fn source() -> Option<impl Iterator<Item = Self::Item>> {
        Some(templates().filter_map(|x| match x {
            gd::Template::GameObject(x) => Some(x),
            _ => None,
        }))
    }
}

#[derive(Default)]
pub(crate) struct SceneryCategory;
impl TableItemCategory for SceneryCategory {
    type Item = &'static SceneryTemplate;

    fn source() -> Option<impl Iterator<Item = Self::Item>> {
        Some(templates().filter_map(|x| match x {
            gd::Template::Scenery(x) => Some(x),
            _ => None,
        }))
    }
}

#[derive(GameObject)]
pub(crate) struct Item {
    pub template: &'static ItemTemplate,
    #[column(name = "GUID")]
    pub id: Option<&'static str>,
    #[column(name = "Internal Name", visible)]
    pub name: &'static str,
    #[column(name = "Display Name", visible)]
    pub display_name: Option<&'static str>,
    #[column(name = "Description")]
    pub desc: Option<&'static str>,
}

impl From<&'static ItemTemplate> for Item {
    fn from(value: &'static ItemTemplate) -> Self {
        Self {
            template: value,
            id: value.id.get().map(|x| x.as_str()),
            name: value.name.as_str(),
            display_name: value.display_name.get().map(|x| x.as_str()),
            desc: value.description.get().map(|x| x.as_str()),
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

    fn source() -> Option<impl Iterator<Item = Self::Item>> {
        Some(templates().filter_map(|x| match x {
            gd::Template::Item(x) => Some(x.into()),
            _ => None,
        }))
    }

    fn draw_actions(&mut self, ui: &Ui, item: &mut Self::Item) {
        if let Some(id) = item.id {
            ui.input_int("Amount", &mut self.give_amount).build();
            if self.give_amount < 1 {
                self.give_amount = 1;
            }

            if ui.button("Give") {
                if let Err(err) = give_item(id, self.give_amount) {
                    err!("failed to give item: {err}");
                };
            }
        }
    }
}
