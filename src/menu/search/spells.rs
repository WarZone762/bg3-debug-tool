use imgui::Ui;

use super::{object_data_tbl, ObjectField, ObjectTableItem};
use crate::{game_definitions::SpellPrototype, globals::Globals};

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

impl Spell {
    pub fn render(&mut self, ui: &Ui) {
        object_data_tbl(ui, |row| {
            if let Some(display_name) = &self.display_name {
                row("Display Name", display_name);
            }
            if let Some(desc) = &self.display_name {
                row("Description", desc);
            }
        })
    }
}

impl ObjectTableItem for Spell {
    type Options = ();

    fn fields() -> Box<[Box<dyn super::TableValueGetter<Self>>]> {
        Box::new([
            ObjectField::getter("Display Name", true, |x| &x.display_name),
            ObjectField::getter("Description", false, |x| &x.desc),
        ])
    }

    fn source() -> impl Iterator<Item = Self> {
        let spell_manager = *Globals::static_symbols().eoc__SpellPrototypeManager.unwrap();
        spell_manager.as_ref().spells.iter().map(|x| x.as_ref().into())
    }
}
