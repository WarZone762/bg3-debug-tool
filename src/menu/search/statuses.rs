use game_object::GameObject;

use super::{
    osiris_helpers::{add_status, remove_status},
    table::TableItemCategory,
};
use crate::{
    err,
    game_definitions::{FixedString, StatusPrototype},
    globals::Globals,
};

#[derive(Clone, GameObject)]
pub(crate) struct Status {
    pub status: &'static StatusPrototype,
    #[column(name = "Intenrnal Name", visible)]
    pub name: Option<String>,
    #[column(name = "Display Name", visible)]
    pub display_name: Option<String>,
    #[column(name = "Description")]
    pub desc: Option<String>,
}

impl From<(&FixedString, &'static StatusPrototype)> for Status {
    fn from(value: (&FixedString, &'static StatusPrototype)) -> Self {
        let name = value.0.get().map(|x| x.to_string());
        let display_name = value.1.description.display_name.try_into().ok();
        let desc = value.1.description.description.try_into().ok();

        Self { status: value.1, name, display_name, desc }
    }
}

pub(crate) struct StatusCategory {
    duration: i32,
}

impl Default for StatusCategory {
    fn default() -> Self {
        Self { duration: -1 }
    }
}

impl StatusCategory {
    fn draw_buttons(&mut self, ui: &imgui::Ui, item: &mut Status) -> anyhow::Result<()> {
        if let Some(name) = &item.name {
            ui.input_int("Seconds (-1 = forever)", &mut self.duration).build();
            if self.duration < -1 {
                self.duration = -1;
            }

            if ui.button("Add") {
                add_status(name, self.duration)?;
            }
            ui.same_line();
            if ui.button("Remove") {
                remove_status(name)?;
            }
        }
        Ok(())
    }
}

impl TableItemCategory for StatusCategory {
    type Item = Status;

    fn source() -> Option<impl Iterator<Item = Self::Item>> {
        let status_manager = Globals::static_symbols().eoc__StatusPrototypeManager.unwrap();
        Some(
            status_manager
                .as_opt()?
                .as_opt()
                .filter(|x| x.initialized)?
                .statuses
                .iter()
                .filter_map(|(name, status)| Some((name, status.as_opt()?).into())),
        )
    }

    fn draw_actions(&mut self, ui: &imgui::Ui, item: &mut Self::Item) {
        if let Err(e) = self.draw_buttons(ui, item) {
            err!("failed to add status: {e}");
        }
    }
}
