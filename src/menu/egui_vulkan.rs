use std::ops::DerefMut;

use egui_demo_lib::DemoWindows;

#[derive(Default)]
pub(crate) struct Menu {
    demo_windows: DemoWindows,
}

impl EguiMenu for Menu {
    fn draw(&mut self, ctx: &egui::Context) {
        self.demo_windows.ui(ctx);
        // egui::Window::new("Test").show(ctx, |ui| {
        //     ui.label("Hello, World!");
        //     if ui.button("Click me").clicked() {}
        // });
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
