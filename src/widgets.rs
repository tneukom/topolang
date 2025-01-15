use crate::{
    brush::Brush,
    material::Material,
    math::rgba8::Rgba8,
    palettes::{Palette, SystemPalette},
    utils::ReflectEnum,
};
use itertools::Itertools;
use std::{ffi::OsStr, fs, path::PathBuf};

pub fn rgba_button(ui: &mut egui::Ui, rgba8: Rgba8, selected: bool) -> egui::Response {
    let egui_color = egui::Color32::from_rgba_unmultiplied(rgba8.r, rgba8.g, rgba8.b, rgba8.a);

    let button = egui::Button::new("").fill(egui_color).selected(selected);
    ui.add_sized([28.0, 28.0], button)
}

pub fn palette_widget(ui: &mut egui::Ui, palette: &Palette, rgba: &mut Rgba8) -> bool {
    let mut color_set = false;

    // 8 colors per row
    for chunk in palette.colors.chunks(4) {
        ui.horizontal(|ui| {
            for choice in chunk {
                if rgba_button(ui, *choice, choice == rgba).clicked() {
                    *rgba = *choice;
                    color_set = true;
                }
            }
        });
    }

    // Link to palette
    ui.hyperlink_to("Link", &palette.link);
    color_set
}

fn palette_chooser(ui: &mut egui::Ui) -> &'static Palette {
    // Id local to the widget
    // let palette_memory_id = ui.make_persistent_id("active_palette");

    // Global Id
    let palette_memory_id = egui::Id::new("active_palette");

    let mut active_palette: usize = ui
        .memory(|memory| memory.data.get_temp(palette_memory_id))
        .unwrap_or(0);
    let palettes = Palette::palettes();

    // Combobox is a popup and in egui only opens one popup at a time.
    // (https://docs.rs/egui/0.29.1/egui/struct.Memory.html#method.toggle_popup)
    // Which doesn't work when the color chooser itself is a popup.
    // egui::ComboBox::from_label("Palettes")
    //     .selected_text(&palette.name)
    //     .show_ui(ui, |ui| {
    //         for (i, palette) in palettes.iter().enumerate() {
    //             ui.selectable_value(&mut active_palette, i, &palette.name);
    //         }
    //     });

    // Show a list of palette buttons instead
    ui.horizontal_wrapped(|ui| {
        for (i_palette, palette) in palettes.iter().enumerate() {
            ui.selectable_value(&mut active_palette, i_palette, &palette.name);
        }
    });

    ui.memory_mut(|memory| memory.data.insert_temp(palette_memory_id, active_palette));
    &palettes[active_palette]
}

pub fn color_chooser(ui: &mut egui::Ui, color: &mut Rgba8) -> bool {
    let palette = palette_chooser(ui);

    // Palette itself
    palette_widget(ui, &palette, color)
}

pub fn system_colors_widget(ui: &mut egui::Ui, rgba: &mut Rgba8) -> bool {
    let mut color_set = false;
    for system_color in SystemPalette::ALL {
        ui.horizontal(|ui| {
            if rgba_button(ui, system_color.rgba(), system_color.rgba() == *rgba).clicked() {
                *rgba = system_color.rgba();
                color_set = true;
            }
            ui.label(system_color.as_str());
        });
    }

    color_set
}

pub fn brush_chooser(ui: &mut egui::Ui, brush: &mut Brush) {
    // Keep solid flag to reapply it later
    let mut is_solid = brush.material.is_solid();

    // Brush color
    let mut color = brush.material.to_rgba();
    if color_chooser(ui, &mut color) {
        brush.material = Material::from(color);
    }

    ui.label("System colors");
    if system_colors_widget(ui, &mut color) {
        brush.material = Material::from(color);
    }

    // Solid checkbox
    ui.add_enabled_ui(
        brush.material.is_normal() || brush.material.is_solid(),
        |ui| {
            if ui.checkbox(&mut is_solid, "Solid").clicked() {
                if is_solid {
                    brush.material = brush.material.as_solid();
                } else {
                    brush.material = brush.material.as_normal();
                }
            }
        },
    );

    // Brush shape
    ui.add(egui::Slider::new(&mut brush.width, 1..=6).text("Radius"));
    ui.separator();
}

pub fn enum_combo<T: ReflectEnum + PartialEq + 'static>(
    ui: &mut egui::Ui,
    title: &str,
    current: &mut T,
) {
    egui::ComboBox::from_label(title)
        .selected_text(current.as_str())
        .width(150.0)
        .show_ui(ui, |ui| {
            for &candidate in T::all() {
                ui.selectable_value(current, candidate, candidate.as_str());
            }
        });
}

pub struct FileChooser {
    file_name: String,
    current_folder: PathBuf,
}

impl FileChooser {
    pub fn new(current_folder: PathBuf) -> Self {
        Self {
            file_name: String::new(),
            current_folder,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> PathBuf {
        // parent folder
        if ui.add(egui::Button::new("../").wrap()).clicked() {
            self.current_folder.pop();
        }

        // TODO: Probably better ways than reading the folder every frame
        let dir_entries: Vec<PathBuf> = fs::read_dir(&self.current_folder)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect();

        // subfolder list
        let sub_folders = dir_entries.iter().filter(|path| path.is_dir());
        let sub_folder_names = sub_folders
            .into_iter()
            .filter_map(|path| path.file_name())
            .filter_map(|osstr| osstr.to_str())
            .sorted();

        for subfolder_name in sub_folder_names {
            if ui
                .add(egui::Button::new(format!("{}/", subfolder_name)).wrap())
                .clicked()
            {
                self.current_folder.push(subfolder_name);
            }
        }

        // files ending in ".png" in folder "resources/saves"
        let extension = OsStr::new("png");
        let files: Vec<_> = dir_entries
            .iter()
            .filter(|path| path.extension() == Some(extension))
            .collect();

        let file_names: Vec<_> = files
            .iter()
            .filter_map(|path| path.file_name())
            .filter_map(|filename| filename.to_str())
            .sorted()
            .collect();

        for file_name in file_names {
            if ui.add(egui::Button::new(file_name).wrap()).clicked() {
                self.file_name = file_name.to_string();
            }
        }

        ui.text_edit_singleline(&mut self.file_name);

        self.current_folder.join(&self.file_name)
    }
}
