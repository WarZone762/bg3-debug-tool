use super::{templates, ObjectField, ObjectTableItem, SearchItem};
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
    type ActionMenu = ();
    type Options = ();

    fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
        Box::new([
            ObjectField::define("Internal Name", true, for<'a> |x: &'a Self| -> &'a str {
                &x.name
            }),
            ObjectField::define("GUID", false, for<'a> |x: &'a Self| -> &'a str { &x.id }),
            ObjectField::define("Display Name", true, for<'a> |x: &'a Self| -> Option<&'a str> {
                x.display_name.as_deref()
            }),
            ObjectField::define("Type", false, for<'a> |x: &'a Self| -> &'a str { &x.r#type }),
        ])
    }

    fn source() -> impl Iterator<Item = Self> {
        templates().filter_map(|x| match x {
            SearchItem::Other(x) => Some(x),
            _ => None,
        })
    }
}
