use std::ops::DerefMut;

use egui_demo_lib::DemoWindows;

use crate::{game_definitions, globals::Globals};

#[derive(Default)]
pub(crate) struct Menu {
    demo_windows: DemoWindows,
    selected: usize,
    size: f32,
}

impl EguiMenu for Menu {
    fn draw(&mut self, ctx: &egui::Context) {
        // self.demo_windows.ui(ctx);
        egui::Window::new("Test").resizable(true).max_size([1024.0, 1536.0]).show(ctx, |ui| {
            ui.input(|i| self.size = (self.size * i.zoom_delta()).clamp(128.0, 4096.0));
            if let Some(atlases) = Globals::static_symbols()
                .ls__gTextureAtlasMap
                .and_then(|x| x.as_opt())
                .and_then(|x| x.as_opt())
                && let Some(item_manager) = Globals::static_symbols()
                    .ls__GlobalTemplateManager
                    .and_then(|x| x.as_opt())
                    .and_then(|x| x.as_opt())
                    .and_then(|x| x.global_template_bank().as_opt())
            {
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        egui::Grid::new("textures").spacing([0.0, 0.0]).max_col_width(256.0).show(
                            ui,
                            |ui| {
                                for (i, item) in item_manager
                                    .templates
                                    .iter()
                                    .filter_map(|x| {
                                        if let game_definitions::Template::Item(x) =
                                            x.value.as_ref().into()
                                        {
                                            return Some(x);
                                        }
                                        None
                                    })
                                    .take(1000)
                                    .enumerate()
                                {
                                    if let Some(atlas) = atlases
                                        .icon_map
                                        .iter()
                                        .find(|x| x.key == *item.icon)
                                        .map(|x| x.value)
                                        && let Some(uvs) = atlas
                                            .icons
                                            .iter()
                                            .find(|x| x.key == *item.icon)
                                            .map(|x| x.value)
                                    {
                                        egui::Frame::default().show(ui, |ui| {
                                            ui.add_sized(
                                                [64.0, 64.0],
                                                egui::Image::new((
                                                    egui::TextureId::User(item.icon.index as _),
                                                    egui::Vec2::new(64.0, 64.0),
                                                ))
                                                .uv([
                                                    [uvs.u1, uvs.v1].into(),
                                                    [uvs.u2, uvs.v2].into(),
                                                ]),
                                            );
                                        });
                                    }
                                    if ui
                                        .selectable_label(
                                            self.selected == i,
                                            item.display_name.get().as_deref().unwrap_or(""),
                                        )
                                        .clicked()
                                    {
                                        self.selected = i;
                                    }
                                    ui.end_row();
                                }
                            },
                        );
                        ui.allocate_space(ui.available_size());
                    });
            }
        });
    }
}

pub(crate) trait EguiMenu {
    fn draw(&mut self, ctx: &egui::Context);
}

impl<M: EguiMenu + ?Sized> EguiMenu for Box<M> {
    fn draw(&mut self, ctx: &egui::Context) {
        self.deref_mut().draw(ctx)
    }
}
