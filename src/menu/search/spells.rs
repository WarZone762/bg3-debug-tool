use game_object::GameObject;

use super::{
    osiris_helpers::{add_spell, add_spell_boost, remove_spell, remove_spell_boost},
    table::TableItemCategory,
};
use crate::{
    err,
    game_definitions::{FixedString, SpellPrototype},
    globals::Globals,
};

#[derive(Clone, GameObject)]
pub(crate) struct Spell {
    pub spell: &'static SpellPrototype,
    #[column(name = "Internal Name", visible)]
    pub name: Option<String>,
    #[column(name = "Display Name", visible)]
    pub display_name: Option<String>,
    #[column(name = "Description")]
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

impl SpellCategory {
    fn draw_buttons(&self, ui: &imgui::Ui, item: &mut Spell) -> anyhow::Result<()> {
        if let Some(name) = &item.name {
            if ui.button("Add for Action") {
                add_spell(name)?;
            }
            ui.same_line();
            if ui.button("Remove for Action") {
                remove_spell(name)?;
            }
            if ui.button("Add for Spell Slot") {
                add_spell_boost(name)?;
            }
            ui.same_line();
            if ui.button("Remove for Spell Slot") {
                remove_spell_boost(name)?;
            }
        }
        Ok(())
    }
}

impl TableItemCategory for SpellCategory {
    type Item = Spell;

    fn source() -> Option<impl Iterator<Item = Self::Item>> {
        let spell_manager = Globals::static_symbols().eoc__SpellPrototypeManager.unwrap();
        Some(
            spell_manager
                .as_opt()?
                .as_opt()
                .filter(|x| x.initialized)?
                .spells
                .iter()
                .filter_map(|(name, spell)| Some((name, spell.as_opt()?).into())),
        )
    }

    fn draw_actions(&mut self, ui: &imgui::Ui, item: &mut Self::Item) {
        if let Err(e) = self.draw_buttons(ui, item) {
            err!("failed to add spell: {e}");
        }
    }
}
