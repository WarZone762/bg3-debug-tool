use imgui::Ui;

use super::{object_data_tbl, templates, ObjectField, ObjectTableItem, SearchItem};
use crate::game_definitions::{EoCGameObjectTemplate, GameObjectTemplate};

#[derive(Debug, Clone)]
pub(crate) struct Other {
    name: String,
    id: String,
    display_name: Option<String>,
    r#type: String,
}

impl From<&EoCGameObjectTemplate> for Other {
    fn from(value: &EoCGameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.into();
        let display_name = (*value.display_name).try_into().ok();

        Self { name, id, display_name, r#type: value.get_type().into() }
    }
}

impl From<&GameObjectTemplate> for Other {
    fn from(value: &GameObjectTemplate) -> Self {
        let name = value.name.to_string();
        let id = value.id.into();

        Self { name, id, display_name: None, r#type: value.get_type().into() }
    }
}

impl ObjectTableItem for Other {
    type Options = ();

    fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
        Box::new([
            ObjectField::getter("Internal Name", true, |x| &x.name),
            ObjectField::getter("GUID", false, |x| &x.id),
            ObjectField::getter("Display Name", true, |x| &x.display_name),
            ObjectField::getter("Type", false, |x| &x.r#type),
        ])
    }

    fn source() -> impl Iterator<Item = Self> {
        templates().filter_map(|x| match x {
            SearchItem::Other(x) => Some(x),
            _ => None,
        })
    }
}

impl Other {
    pub fn render(&mut self, ui: &Ui) {
        object_data_tbl(ui, |row| {
            row("Type", &self.r#type);
            row("GUID", &self.id);
            row("Name", &self.name);
            if let Some(display_name) = &self.display_name {
                row("Display Name", display_name);
            }
        })
    }
}
