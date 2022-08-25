use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::{sorting::Threshold, Settings};

pub(crate) fn ui(mut egui_context: ResMut<EguiContext>, mut settings: ResMut<Settings>) {
    egui::Window::new("Settings")
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ComboBox::from_label("Select one!")
                .selected_text(format!("{}", settings.threshold))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut settings.threshold,
                        Threshold::Luminance(150.),
                        "Luminance",
                    );
                    ui.selectable_value(&mut settings.threshold, Threshold::Red(150), "Red");
                    ui.selectable_value(&mut settings.threshold, Threshold::Green(150), "Green");
                    ui.selectable_value(&mut settings.threshold, Threshold::Blue(150), "Blue");
                });

            match settings.threshold {
                Threshold::Luminance(ref mut val) => {
                    ui.add(
                        egui::Slider::new(val, 0.0..=255.0)
                            .text("Luminance")
                            .step_by(0.01),
                    );
                }
                Threshold::Red(ref mut val)
                | Threshold::Green(ref mut val)
                | Threshold::Blue(ref mut val) => {
                    ui.add(
                        egui::Slider::new(val, 0..=255)
                            .text("Luminance")
                            .step_by(0.1),
                    );
                }
            }
            ui.checkbox(&mut settings.ordering_reverse, "Reverse Ordering");
            ui.checkbox(&mut settings.threshold_reverse, "Reverse Threshold");
        });
}
