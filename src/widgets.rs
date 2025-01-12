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

pub fn palette_widget(ui: &mut egui::Ui, palette: &Palette, rgba: &mut Rgba8) {
    ui.horizontal_wrapped(|ui| {
        for &choice in &palette.colors {
            if rgba_button(ui, choice, &choice == rgba).clicked() {
                *rgba = choice;
            }
        }
    });

    // Link to palette
    ui.hyperlink_to("Link", &palette.link);
}

pub fn system_colors_widget(ui: &mut egui::Ui, rgba: &mut Rgba8) {
    for system_color in SystemPalette::ALL {
        ui.horizontal(|ui| {
            if rgba_button(ui, system_color.rgba(), system_color.rgba() == *rgba).clicked() {
                *rgba = system_color.rgba();
            }
            ui.label(system_color.as_str());
        });
    }
}

pub struct ColorChooser {
    pub palettes: Vec<Palette>,
    pub current_palette: usize,
    pub color: Rgba8,
}

impl ColorChooser {
    pub fn new(palettes: Vec<Palette>) -> Self {
        assert!(palettes.len() > 0);
        let color = palettes[0].colors[0];
        Self {
            palettes,
            current_palette: 0,
            color,
        }
    }

    pub fn default() -> Self {
        let palettes = vec![
            Palette::palette_pico8(),
            Palette::palette_windows16(),
            Palette::palette_na16(),
            Palette::palette_desatur8(),
            Palette::palette_hept32(),
            Palette::palette_superfuture25(),
        ];
        Self::new(palettes)
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Drop down of palettes
        egui::ComboBox::from_label("Palettes")
            .selected_text(&self.palettes[self.current_palette].name)
            .show_ui(ui, |ui| {
                for (i, palette) in self.palettes.iter().enumerate() {
                    ui.selectable_value(&mut self.current_palette, i, &palette.name);
                }
            });

        // Palette itself
        palette_widget(ui, &self.palettes[self.current_palette], &mut self.color);
    }
}

pub struct BrushChooser {
    color_chooser: ColorChooser,
    solid: bool,
    radius: i64,
}

impl BrushChooser {
    pub fn new(color_chooser: ColorChooser) -> Self {
        Self {
            color_chooser,
            solid: false,
            radius: 1,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Brush color
        self.color_chooser.show(ui);

        ui.label("System colors");
        system_colors_widget(ui, &mut self.color_chooser.color);

        // Solid checkbox
        ui.add_enabled_ui(self.color_chooser.color.a == 255, |ui| {
            ui.checkbox(&mut self.solid, "Solid");
        });

        // Brush shape
        ui.add(egui::Slider::new(&mut self.radius, 1..=6).text("Radius"));
        ui.separator();
    }

    pub fn material(&self) -> Material {
        let material = Material::from(self.color_chooser.color);
        if material.is_normal() && self.solid {
            material.as_solid()
        } else {
            material
        }
    }

    pub fn brush(&self) -> Brush {
        Brush {
            material: self.material(),
            radius: self.radius,
        }
    }
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
