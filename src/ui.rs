use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::{
    sorting::{PixelOrdering, Threshold},
    Settings,
};

pub(crate) fn ui(
    mut egui_context: ResMut<EguiContext>,
    mut settings: ResMut<Settings>,
    mut commands: Commands,
) {
    //  commands.add(RegisterStandardDynamicAsset {
    //         key: "character",
    //         asset: StandardDynamicAsset::File {
    //             path: "https://i.laundmo.com/tENe0/CavaMUlo03.jpg".to_owned(),
    //         },
    //     });
    egui::Window::new("Settings")
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    threshold_ui(&mut settings, ui);
                    ordering_ui(&mut settings, ui);
                })
        });
}

const DEFAULT_THRESHOLDS: [Threshold; 2] = [
    Threshold::Luminance(150.),
    Threshold::ColorSimilarity(1000, [0, 255, 0]),
];

fn threshold_ui(settings: &mut ResMut<Settings>, ui: &mut egui::Ui) {
    ui.label("Threshold:");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_source("thresh")
            .selected_text(format!("{}", settings.threshold))
            .show_ui(ui, |ui| {
                for default in DEFAULT_THRESHOLDS {
                    let name = format!("{}", default);
                    ui.selectable_value(&mut settings.threshold, default, name);
                }
            });
        ui.toggle_value(&mut settings.threshold_reverse, "Invert");
    });
    ui.end_row();
    ui.label("Threshold Values:");
    ui.horizontal(|ui| match settings.threshold {
        Threshold::Luminance(ref mut val) => {
            ui.add(
                egui::DragValue::new(val)
                    .clamp_range(0.0..=255.0)
                    .speed(0.1),
            );
            ui.end_row();
        }
        Threshold::ColorSimilarity(ref mut val, ref mut color) => {
            ui.add(egui::DragValue::new(val).clamp_range(0..=2500).speed(1.0));
            ui.color_edit_button_srgb(color);
        }
    });
    ui.end_row();
    ui.label("Extend:");
    ui.horizontal(|ui| {
        ui.label("Left:");
        ui.add(
            egui::DragValue::new(&mut settings.extend_threshold_left)
                .clamp_range(0..=500)
                .speed(1.),
        );
        ui.label("Right:");
        ui.add(
            egui::DragValue::new(&mut settings.extend_threshold_right)
                .clamp_range(0..=500)
                .speed(1.),
        );
    });
    ui.end_row();
}

const DEFAULT_ORDERINGS: [PixelOrdering; 2] = [
    PixelOrdering::Luminance,
    PixelOrdering::ColorSimilarity([0, 255, 0]),
];

fn ordering_ui(settings: &mut ResMut<Settings>, ui: &mut egui::Ui) {
    ui.label("Ordering:");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_source("sortby")
            .selected_text(format!("{}", settings.ordering))
            .show_ui(ui, |ui| {
                for default in DEFAULT_ORDERINGS {
                    let name = format!("{}", default);
                    ui.selectable_value(&mut settings.ordering, default, name);
                }
            });
        ui.toggle_value(&mut settings.ordering_reverse, "Reverse");
    });
    ui.end_row();
    ui.label("");
    match settings.ordering {
        PixelOrdering::Luminance => (),
        PixelOrdering::ColorSimilarity(ref mut color) => {
            ui.color_edit_button_srgb(color);
            ui.end_row();
        }
    }
}
