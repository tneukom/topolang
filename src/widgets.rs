use itertools::Itertools;
use std::{ffi::OsStr, fs, ops::RangeInclusive, path::PathBuf};

use crate::{math::rgba8::Rgba8, utils::ReflectEnum};

pub fn load_palette(path: &str) -> Option<Vec<Rgba8>> {
    use image::{io::Reader, DynamicImage::ImageRgba8};

    // TODO: Better error forwarding
    if let ImageRgba8(img) = Reader::open(path).ok()?.decode().ok()? {
        Some(
            img.pixels()
                .map(|img_rgba| Rgba8::new(img_rgba[0], img_rgba[1], img_rgba[2], img_rgba[3]))
                .collect(),
        )
    } else {
        None
    }
}

pub fn opt_drag_value<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    opt_value: &mut Option<Num>,
    range: RangeInclusive<Num>,
) {
    let mut has_value = opt_value.is_some();

    ui.horizontal(|ui| {
        ui.checkbox(&mut has_value, "");

        if has_value {
            let mut value = opt_value.unwrap_or(*range.start());
            ui.add(egui::DragValue::new(&mut value).clamp_range(range));
            *opt_value = Some(value);
        } else {
            *opt_value = None;
        }
    });
}

pub fn palette_widget(ui: &mut egui::Ui, palette: &Vec<Rgba8>, rgba: &mut Rgba8) {
    // 8 colors per row
    for chunk in palette.chunks(8) {
        ui.horizontal(|ui| {
            for choice in chunk {
                let egui_color =
                    egui::Color32::from_rgba_unmultiplied(choice.r, choice.g, choice.b, choice.a);
                let response = ui.add_sized([20.0, 20.0], egui::Button::new("").fill(egui_color));
                if response.clicked() {
                    *rgba = *choice;
                }
            }
        });
    }
}

// pub fn Rational_slider(
//     ui: &mut egui::Ui,
//     value: &mut Rational,
//     denom: u64,
//     num_range: RangeInclusive<i64>,
// ) {
//     // assume arrow.head_size is a multiple of 1/10
//     let mut num = (denom * value.into()).round();
//     ui.add(egui::Slider::new(&mut num, num_range).text("Head Size"));
//     *value = Rational::from_parts(num.into(), denom.into());
// }

// pub fn color_picker_srgba(ui: &mut egui::Ui, rgba8: &mut Rgba8) -> bool {
//     // Convert srgba -> linear rgba -> hsva, call the color picker and convert back
//     let mut linear_rgba = egui::Rgba::from_srgba_unmultiplied(rgba8.r, rgba8.g, rgba8.b, rgba8.a);
//     let mut hsva = egui::color::Hsva::from(linear_rgba);
//     let changed = egui::color_picker::color_picker_hsva_2d(ui, &mut hsva, egui::color_picker::Alpha::OnlyBlend);
//     linear_rgba = egui::Rgba::from(hsva);
//     *rgba8 = Rgba8::from(linear_rgba.to_srgba_unmultiplied());
//     changed
// }
pub fn rgba_button(ui: &mut egui::Ui, palette: &Vec<Rgba8>, rgba: &mut Rgba8) -> egui::Response {
    let egui_color = egui::Rgba::from_srgba_unmultiplied(rgba.r, rgba.g, rgba.b, rgba.a);
    let response = ui.add_sized([40.0, 20.0], egui::Button::new("").fill(egui_color));

    // let response = ui.button("Select color");
    let popup_id = ui.make_persistent_id("rgba_popup");
    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup_id));
    }

    egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
        palette_widget(ui, palette, rgba);

        // rgb sliders
        ui.add(egui::Slider::new(&mut rgba.r, 0..=255).text("Red"));
        ui.add(egui::Slider::new(&mut rgba.g, 0..=255).text("Green"));
        ui.add(egui::Slider::new(&mut rgba.b, 0..=255).text("Blue"));
        ui.add(egui::Slider::new(&mut rgba.a, 0..=255).text("Alpha"));

        // Self::color_picker_srgba(ui, rgba);
    });

    // let mut rgbaf = rgba.to_f32();
    // let response = ui.color_edit_button_rgba_unmultiplied(&mut rgbaf);
    // *rgba = Rgba8::from(&rgbaf);
    response
}

pub fn optional_rgba(
    ui: &mut egui::Ui,
    label: &str,
    palette: &Vec<Rgba8>,
    has_value: &mut bool,
    color: &mut Rgba8,
) {
    ui.horizontal(|ui| {
        ui.checkbox(has_value, label);

        // Color
        ui.add_enabled_ui(*has_value, |ui| {
            rgba_button(ui, palette, color);
        });

        if *has_value {
            // Show label with hex color
            ui.label(color.hex());
        }
    });
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
        // File & dir list
        let geo_extension = OsStr::new("tiles");

        // parent folder
        if ui.add(egui::Button::new("../").wrap(false)).clicked() {
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
                .add(egui::Button::new(format!("{}/", subfolder_name)).wrap(false))
                .clicked()
            {
                self.current_folder.push(subfolder_name);
            }
        }

        // files ending in ".geo" in folder "resources/saves"
        let files: Vec<_> = dir_entries
            .iter()
            .filter(|path| path.extension() == Some(&geo_extension))
            .collect();

        let file_names: Vec<_> = files
            .iter()
            .filter_map(|path| path.file_name())
            .filter_map(|filename| filename.to_str())
            .sorted()
            .collect();

        for file_name in file_names {
            if ui.add(egui::Button::new(file_name).wrap(false)).clicked() {
                self.file_name = file_name.to_string();
            }
        }

        ui.text_edit_singleline(&mut self.file_name);

        self.current_folder.join(&self.file_name)
    }
}
