use super::{templates2, SearchItem, TableColumn, TableItem, TableItemCategory};
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

impl TableItem for Other {
    fn columns() -> Box<[super::TableColumn]> {
        Box::new([
            TableColumn::new("Internal Name", true, true),
            TableColumn::new("GUID", false, false),
            TableColumn::new("Display Name", true, true),
            TableColumn::new("Type", true, true),
        ])
    }

    fn draw(&self, ui: &imgui::Ui, i: usize) {
        match i {
            0 => super::TableValue::draw(&self.name, ui),
            1 => super::TableValue::draw(&self.id, ui),
            2 => super::TableValue::draw(&self.display_name, ui),
            3 => super::TableValue::draw(&self.r#type, ui),
            _ => unreachable!(),
        }
    }

    fn search_str(&self, i: usize) -> String {
        match i {
            0 => super::TableValue::search_str(&self.name),
            1 => super::TableValue::search_str(&self.id),
            2 => super::TableValue::search_str(&self.display_name),
            3 => super::TableValue::search_str(&self.r#type),
            _ => unreachable!(),
        }
    }

    fn compare(&self, other: &Self, i: usize) -> std::cmp::Ordering {
        match i {
            0 => super::TableValue::compare(&self.name, &other.name),
            1 => super::TableValue::compare(&self.id, &other.id),
            2 => super::TableValue::compare(&self.display_name, &other.display_name),
            3 => super::TableValue::compare(&self.r#type, &other.r#type),
            _ => unreachable!(),
        }
    }
}

#[derive(Default)]
pub(crate) struct OtherCategory;
impl TableItemCategory for OtherCategory {
    type Item = Other;

    fn source() -> impl Iterator<Item = Self::Item> {
        templates2().filter_map(|x| match x {
            SearchItem::Other(x) => Some(x),
            _ => None,
        })
    }
}

// impl ObjectTableItem for Other {
//     type ActionMenu = ();
//     type Options = ();
//
//     fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
//         Box::new([
//             ObjectField::define("Internal Name", true, true, for<'a> |x: &'a
// Self| -> &'a str {                 &x.name
//             }),
//             ObjectField::define("GUID", false, false, for<'a> |x: &'a Self|
// -> &'a str { &x.id }),             ObjectField::define(
//                 "Display Name",
//                 true,
//                 true,
//                 for<'a> |x: &'a Self| -> Option<&'a str> {
// x.display_name.as_deref() },             ),
//             ObjectField::define("Type", true, false, for<'a> |x: &'a Self| ->
// &'a str {                 &x.r#type
//             }),
//         ])
//     }
//
//     // fn source() -> impl Iterator<Item = Self> {
//     //     templates().filter_map(|x| match x {
//     //         SearchItem::Other(x) => Some(x),
//     //         _ => None,
//     //     })
//     // }
// }
