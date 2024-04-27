use game_object::GameObject;

use super::table::TableItemCategory;
use crate::{
    game_definitions::{FixedString, SpellPrototype},
    globals::Globals,
    info,
};

#[derive(GameObject, Clone)]
pub(crate) struct Spell {
    pub spell: &'static SpellPrototype,
    #[column(name = "ID", visible)]
    pub name: Option<String>,
    #[column(name = "Name", visible)]
    pub display_name: Option<String>,
    #[column(name = "Description", visible)]
    pub desc: Option<String>,
}

impl From<(&FixedString, &'static SpellPrototype)> for Spell {
    fn from(value: (&FixedString, &'static SpellPrototype)) -> Self {
        let name = value.0.get().map(|x| x.to_string());
        let display_name = value.1.description.display_name.try_into().ok();
        let desc = value.1.description.description.try_into().ok();

        Self { spell: value.1, name, display_name, desc }
    }
}

#[derive(Default)]
pub(crate) struct SpellCategory;
impl TableItemCategory for SpellCategory {
    type Item = Spell;

    fn source() -> impl Iterator<Item = Self::Item> {
        let spell_manager = Globals::static_symbols().eoc__SpellPrototypeManager.unwrap();
        spell_manager
            .as_opt()
            .and_then(|x| x.as_opt())
            .filter(|x| x.initialized)
            .into_iter()
            .flat_map(|x| {
                x.spells.iter().filter_map(|(name, spell)| Some((name, spell.as_opt()?).into()))
            })
    }
}
