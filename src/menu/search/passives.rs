use game_object::GameObject;

use super::{
    osiris_helpers::{add_passive, has_passive, is_game_state_running, remove_passive},
    table::TableItemCategory,
};
use crate::{
    err,
    game_definitions::{FixedString, PassivePrototype},
    globals::Globals,
};

#[derive(Clone, GameObject)]
pub(crate) struct Passive {
    pub passive: &'static PassivePrototype,
    #[column(name = "Intenrnal Name", visible)]
    pub name: Option<String>,
    #[column(name = "Display Name", visible)]
    pub display_name: Option<String>,
    #[column(name = "Description")]
    pub desc: Option<String>,
}

impl From<(&FixedString, &'static PassivePrototype)> for Passive {
    fn from(value: (&FixedString, &'static PassivePrototype)) -> Self {
        let name = value.0.get().map(|x| x.to_string());
        let display_name = value.1.description.display_name.try_into().ok();
        let desc = value.1.description.description.try_into().ok();

        Self { passive: value.1, name, display_name, desc }
    }
}

#[derive(Default)]
pub(crate) struct PassiveCategory;

impl PassiveCategory {
    fn draw_buttons(&mut self, ui: &imgui::Ui, item: &mut Passive) -> anyhow::Result<()> {
        if let Some(name) = &item.name {
            if !is_game_state_running().is_ok_and(|x| x) {
                ui.text("Waiting for game to load...");
                ui.disabled(true, || {
                    ui.text("Is present: ");
                    ui.same_line();
                    ui.text_colored([1.0, 0.0, 0.0, 1.0], "");
                    ui.button("Add");
                });
                return Ok(());
            }

            ui.text("Is present: ");
            ui.same_line();
            if !has_passive(name).is_ok_and(|x| x) {
                ui.text_colored([1.0, 0.0, 0.0, 1.0], "");
                if ui.button("Add") {
                    add_passive(name)?;
                }
            } else {
                ui.text_colored([0.0, 1.0, 0.0, 1.0], "");
                if ui.button("Remove") {
                    remove_passive(name)?;
                }
            }
        }
        Ok(())
    }
}

impl TableItemCategory for PassiveCategory {
    type Item = Passive;

    fn source() -> Option<impl Iterator<Item = Self::Item>> {
        let passive_manager = Globals::static_symbols().eoc__PassivePrototypeManager.unwrap();
        Some(
            passive_manager
                .as_opt()?
                .as_opt()
                .filter(|x| x.initialized)?
                .passives
                .iter()
                .filter_map(move |node| {
                    let node = node.as_opt()?;
                    Some((&node.key, &node.value).into())
                }),
        )
    }

    fn draw_actions(&mut self, ui: &imgui::Ui, item: &mut Self::Item) {
        if let Err(e) = self.draw_buttons(ui, item) {
            err!("failed to add passive: {e}");
        }
    }
}
