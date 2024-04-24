use crate::game_definitions::SpellPrototype;

#[derive(Debug, Clone)]
pub(crate) struct Spell {
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

// impl ObjectTableItem for Spell {
//     type ActionMenu = ();
//     type Options = ();
//
//     fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
//         Box::new([
//             ObjectField::define(
//                 "Display Name",
//                 true,
//                 true,
//                 for<'a> |x: &'a Self| -> Option<&'a str> {
// x.display_name.as_deref() },             ),
//             ObjectField::define(
//                 "Description",
//                 true,
//                 false,
//                 for<'a> |x: &'a Self| -> Option<&'a str> { x.desc.as_deref()
// },             ),
//         ])
//     }
//
//     // fn source() -> impl Iterator<Item = Self> {
//     //     let spell_manager =
//     // *Globals::static_symbols().eoc__SpellPrototypeManager.unwrap();
//     //     spell_manager.as_ref().spells.iter().map(|x| x.as_ref().into())
//     // }
// }
