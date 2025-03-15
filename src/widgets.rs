use crate::{
    brush::Brush,
    material::{Material, MaterialClass},
    math::rgba8::Rgba8,
    palettes::Palette,
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
    ui.horizontal_wrapped(|ui| {
        for &choice in &palette.colors {
            if rgba_button(ui, choice, choice == *rgba).clicked() {
                *rgba = choice;
                color_set = true;
            }
        }
    });

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

    // Show a list of palette buttons instead
    ui.horizontal_wrapped(|ui| {
        for (i_palette, palette) in palettes.iter().enumerate() {
            ui.selectable_value(&mut active_palette, i_palette, &palette.name);
        }
    });

    ui.memory_mut(|memory| memory.data.insert_temp(palette_memory_id, active_palette));
    &palettes[active_palette]
}

/// Return true if the color was changed
pub fn color_chooser(ui: &mut egui::Ui, color: &mut Rgba8) -> bool {
    let palette = palette_chooser(ui);

    // Palette itself
    palette_widget(ui, &palette, color)
}

pub fn rgb_chooser(ui: &mut egui::Ui, rgb: &mut [u8; 3]) -> bool {
    let palette = palette_chooser(ui);

    // Palette itself
    let mut rgba = Rgba8::from_rgb(*rgb);
    let changed = palette_widget(ui, &palette, &mut rgba);
    *rgb = rgba.rgb();
    changed
}

/// Returns true if the color was changed
pub fn system_material_widget(ui: &mut egui::Ui, material: &mut Material) -> bool {
    let system_materials = [
        ("Rule Before", Material::RULE_BEFORE),
        ("Rule After", Material::RULE_AFTER),
        ("Wildcard", Material::WILDCARD),
    ];

    let mut color_set = false;
    for (name, system_material) in system_materials {
        ui.horizontal(|ui| {
            if rgba_button(ui, system_material.to_rgba(), system_material == *material).clicked() {
                *material = system_material;
                color_set = true;
            }
            ui.label(name);
        });
    }

    color_set
}

pub fn material_chooser(ui: &mut egui::Ui, material: &mut Material) {
    // Color
    let mut rgb = material.rgb;
    if rgb_chooser(ui, &mut rgb) {
        *material = match material.class {
            MaterialClass::Solid => Material::new(rgb, MaterialClass::Solid),
            MaterialClass::Sleeping => Material::new(rgb, MaterialClass::Sleeping),
            _ => Material::new(rgb, MaterialClass::Normal),
        };
    }

    ui.label("System colors");
    system_material_widget(ui, material);

    // Material classes
    let is_reserved = material.is_rule() || material.is_wildcard();

    let choices = [
        (MaterialClass::Solid, "Solid"),
        (MaterialClass::Normal, "Normal"),
        (MaterialClass::Sleeping, "Sleeping"),
    ];

    segmented_choice(ui, choices, &mut material.class);
}

pub fn brush_chooser(ui: &mut egui::Ui, brush: &mut Brush) {
    material_chooser(ui, &mut brush.material);

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

pub fn segmented_choice<'a, T: Copy + Eq>(
    ui: &mut egui::Ui,
    choices: impl IntoIterator<Item = (T, &'a str)>,
    selected: &mut T,
) -> bool {
    let style = ui.style();
    let frame = egui::Frame {
        inner_margin: egui::Margin::same(6.0),
        rounding: egui::Rounding::same(4.0),
        fill: style.visuals.widgets.inactive.bg_fill,
        ..Default::default()
    };

    let mut clicked = false;
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            for (choice, label) in choices.into_iter() {
                if ui.selectable_value(selected, choice, label).clicked() {
                    clicked = true;
                }
            }
        })
    });

    clicked
}

pub fn segmented_enum_choice<T: Copy + ReflectEnum + Eq>(
    ui: &mut egui::Ui,
    selected: &mut T,
) -> bool {
    let choices = T::all()
        .into_iter()
        .map(|&choice| (choice, choice.as_str()));
    segmented_choice(ui, choices, selected)
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
