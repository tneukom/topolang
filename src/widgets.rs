use crate::{
    brush::Brush,
    field::RgbaField,
    material::{Material, MaterialClass},
    material_effects::{CHECKERBOARD_EVEN_RGBA, CHECKERBOARD_ODD_RGBA, material_map_effects},
    math::{
        point::Point,
        rect::Rect,
        rgba8::{Rgb8, Rgba8},
    },
    palettes::Palette,
    pixmap::MaterialMap,
    rule::InputEvent,
    utils::ReflectEnum,
};
use cached::proc_macro::cached;
use egui::{AtomExt, IntoAtoms};
use itertools::Itertools;
use std::{ffi::OsStr, fs, path::PathBuf, sync::OnceLock};

const COLOR_BUTTON_MARGIN: f32 = 2.0;

fn rgba_field_egui_texture(ui: &mut egui::Ui, rgba_field: &RgbaField) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [rgba_field.width() as usize, rgba_field.height() as usize],
        rgba_field.as_raw(),
    );

    ui.ctx()
        .load_texture("icon_texture", image, Default::default())
}

#[cached(key = "(Material, i64)", convert = r#"{ (material, icon_size) }"#)]
fn material_icon(ui: &mut egui::Ui, material: Material, icon_size: i64) -> egui::TextureHandle {
    let bounds = Rect::low_size(Point(0, 0), Point(icon_size, icon_size));
    let material_map = MaterialMap::filled(bounds, material);

    let mut rgba_field = material_map_effects(&material_map, Rgba8::TRANSPARENT);

    // Checkerboard background with squares of 7 by 7 pixels
    for (pixel, rgba) in rgba_field.enumerate_mut() {
        let background_f32 = if (pixel.x / 7 + pixel.y / 7) % 2 == 0 {
            CHECKERBOARD_EVEN_RGBA.to_f32()
        } else {
            CHECKERBOARD_ODD_RGBA.to_f32()
        };

        let rgba_f32 = rgba.to_f32();
        let blended_rgb_f32 =
            rgba_f32.a * rgba_f32.rgb() + (1.0 - rgba_f32.a) * background_f32.rgb();
        *rgba = Rgba8::from_rgb_a(blended_rgb_f32.to_u8(), 255);
    }
    rgba_field_egui_texture(ui, &rgba_field)
}

/// Warning: A new egui texture is created for each material, and is cached forever.
pub fn material_button(ui: &mut egui::Ui, material: Material, selected: bool) -> egui::Response {
    let icon = material_icon(ui, material, 28);

    let sized_texture = egui::load::SizedTexture::from(&icon);
    let button = egui::widgets::ImageButton::new(sized_texture).selected(selected);
    ui.add(button)
}

// pub fn rgba_button(ui: &mut egui::Ui, rgba8: Rgba8, selected: bool) -> egui::Response {
//     let egui_color = egui::Color32::from_rgba_unmultiplied(rgba8.r, rgba8.g, rgba8.b, rgba8.a);
//
//     let button = egui::Button::new("").fill(egui_color).selected(selected);
//     ui.add_sized([28.0, 28.0], button)
// }

fn palette_btn_style(ui: &mut egui::Ui) {
    // 2 pixel padding and spacing
    ui.style_mut().spacing.button_padding = egui::Vec2::splat(2.0);
    ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

    // Set padding color to same as panel background
    let padding_fill = ui.style_mut().visuals.panel_fill;
    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = padding_fill;
    ui.style_mut().visuals.widgets.active.weak_bg_fill = padding_fill;
    ui.style_mut().visuals.widgets.noninteractive.weak_bg_fill = padding_fill;
}

pub fn palette_widget(ui: &mut egui::Ui, palette: &Palette, rgba: &mut Rgba8) -> bool {
    let mut color_set = false;

    ui.scope(|ui| {
        palette_btn_style(ui);

        // 8 colors per row
        ui.horizontal_wrapped(|ui| {
            for &choice in &palette.colors {
                let choice_material = Material::new(choice.rgb(), MaterialClass::Normal);
                if material_button(ui, choice_material, choice == *rgba).clicked() {
                    *rgba = choice;
                    color_set = true;
                }
                // if rgba_button(ui, choice, choice == *rgba).clicked() {
                //     *rgba = choice;
                //     color_set = true;
                // }
            }
        });
    });

    color_set
}

pub fn styled_button<'a>(atoms: impl IntoAtoms<'a>) -> egui::Button<'a> {
    egui::Button::new(atoms).corner_radius(4)
}

pub fn icon_button<'a>(icon: egui::ImageSource<'a>, size: f32) -> egui::Button<'a> {
    let icon_size = egui::Vec2::splat(size);
    styled_button(icon.atom_size(icon_size))
}

pub fn styled_space(ui: &mut egui::Ui) {
    ui.add_space(6.0);
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
            let button = styled_button(&palette.name).selected(active_palette == i_palette);
            if ui.add(button).clicked() {
                active_palette = i_palette;
            }
        }
    });

    ui.memory_mut(|memory| memory.data.insert_temp(palette_memory_id, active_palette));
    let palette = &palettes[active_palette];

    // Link to palette
    ui.hyperlink_to("Link", &palette.link);

    palette
}

/// Return true if the color was changed
pub fn color_chooser(ui: &mut egui::Ui, color: &mut Rgba8) -> bool {
    let palette = palette_chooser(ui);

    // Palette itself
    palette_widget(ui, &palette, color)
}

pub fn rgb_chooser(ui: &mut egui::Ui, rgb: &mut Rgb8) -> bool {
    let palette = if cfg!(feature = "minimal_ui") {
        &Palette::palettes()[0]
    } else {
        let palette = palette_chooser(ui);
        styled_space(ui);
        palette
    };

    // Palette itself
    let mut rgba = Rgba8::from_rgb(*rgb);
    let changed = palette_widget(ui, &palette, &mut rgba);
    *rgb = rgba.rgb();
    changed
}

/// Returns true if the color was changed
pub fn system_material_widget(ui: &mut egui::Ui, material: &mut Material) -> bool {
    let mut color_set = false;

    let mut btn = |ui: &mut egui::Ui, name: &str, choice| {
        if material_button(ui, choice, choice == *material).clicked() {
            *material = choice;
            color_set = true;
        }
        ui.label(name);
    };

    ui.scope(|ui| {
        palette_btn_style(ui);

        ui.horizontal(|ui| {
            btn(ui, "Rule Before", Material::RULE_BEFORE);
            btn(ui, "Rule After", Material::RULE_AFTER);
        });

        ui.horizontal(|ui| {
            btn(ui, "Wildcard", Material::WILDCARD);
        });

        #[cfg(feature = "link_ui")]
        ui.horizontal(|ui| {
            btn(ui, "Link", Material::LINK);
            btn(ui, "Link Hover", Material::LINK_HOVER);
        });

        ui.horizontal(|ui| {
            btn(ui, "Placeholder", Material::RULE_PLACEHOLDER);
            btn(ui, "Choice", Material::RULE_CHOICE);
        });
    });

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
    styled_space(ui);

    system_material_widget(ui, material);
    styled_space(ui);

    // Material classes
    let is_reserved = material.is_rule() || material.is_wildcard();

    let choices = [
        (MaterialClass::Normal, "Normal"),
        (MaterialClass::Solid, "Solid"),
        (MaterialClass::Sleeping, "Sleeping"),
    ];

    ui.add_enabled_ui(!is_reserved, |ui| {
        ui.horizontal(|ui| {
            choice_buttons(ui, None, choices, &mut material.class);
        });
    });
}

#[cached(
    key = "(Brush, i64, i64)",
    convert = r#"{ (brush, icon_size, upscale) }"#
)]
fn brush_icon(
    ui: &mut egui::Ui,
    brush: Brush,
    icon_size: i64,
    upscale: i64,
) -> egui::TextureHandle {
    // Center at 0 to make upscaling the dot easier
    let bounds = Rect::low_size(Point::ZERO, Point(icon_size, icon_size));
    let mut material_map = MaterialMap::nones(bounds);

    let center = bounds.as_f64().center();
    let dot = brush.dot(center).integer_upscale(upscale);

    // Translate dot to center
    let offset = bounds.as_f64().center() - dot.bounding_rect().as_f64().center();
    let dot = dot.translated(offset.as_i64());

    material_map.blit(&dot);

    let rgba_field = material_map_effects(&material_map, Rgba8::TRANSPARENT);
    rgba_field_egui_texture(ui, &rgba_field)
}

pub fn brush_size_chooser(ui: &mut egui::Ui, size: &mut i64) {
    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.style_mut().spacing.button_padding = egui::Vec2::new(1.0, 1.0);

            for choice in 1..=6 {
                let brush = Brush {
                    material: Material::BLACK,
                    size: choice,
                };
                let icon = brush_icon(ui, brush, 28, 3);

                let sized_texture = egui::load::SizedTexture::from(&icon);
                let button =
                    egui::widgets::ImageButton::new(sized_texture).selected(*size == choice);
                if ui.add(button).clicked() {
                    *size = choice;
                }
            }
        });
    });
}

pub fn brush_chooser(ui: &mut egui::Ui, brush: &mut Brush) {
    // Brush shape
    brush_size_chooser(ui, &mut brush.size);
    ui.add_space(10.0);

    material_chooser(ui, &mut brush.material);
}

struct Prefab {
    material_map: MaterialMap,
    icon: egui::TextureHandle,
}

/// List of buttons with saved bitmaps that can be added to the world.
pub fn prefab_picker(ui: &mut egui::Ui) -> Option<&'static MaterialMap> {
    static PREFABS: OnceLock<Vec<Prefab>> = OnceLock::new();
    let prefabs = PREFABS.get_or_init(|| {
        let mut prefabs = Vec::new();
        for input_event in InputEvent::ALL {
            let png = input_event.symbol_png();
            let rgba_field = RgbaField::load_from_memory(png).unwrap();
            let material_map = MaterialMap::from(rgba_field).map(|material| material.as_normal());

            let icon_rgba =
                material_map_effects(&material_map, Rgba8::TRANSPARENT).integer_upscale(2);
            let icon = rgba_field_egui_texture(ui, &icon_rgba);

            let prefab = Prefab { material_map, icon };

            prefabs.push(prefab);
        }
        prefabs
    });

    let mut picked = None;
    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.style_mut().spacing.button_padding =
                egui::Vec2::new(COLOR_BUTTON_MARGIN, COLOR_BUTTON_MARGIN);

            for prefab in prefabs {
                let sized_texture = egui::load::SizedTexture::from(&prefab.icon);
                let button = egui::widgets::ImageButton::new(sized_texture);
                if ui.add(button).clicked() {
                    picked = Some(&prefab.material_map)
                }
            }
        });
    });

    picked
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

pub fn choice_buttons<'a, T: Copy + Eq>(
    ui: &mut egui::Ui,
    title: Option<&str>,
    choices: impl IntoIterator<Item = (T, &'a str)>,
    selected: &mut T,
) -> bool {
    let mut clicked = false;

    // TODO: Would be nicer if this was left of the buttons, but vertical centering doesn't work
    //   properly because the layout doesn't know the height of the widgets following the labels.
    //   It works if the label is after the buttons.
    if let Some(title) = title {
        ui.label(title);
    }

    for (choice, label) in choices.into_iter() {
        if ui
            .add(styled_button(label).selected(choice == *selected))
            .clicked()
        {
            *selected = choice;
            clicked = true;
        }
    }

    clicked
}

pub fn enum_choice_buttons<T: Copy + ReflectEnum + Eq>(
    ui: &mut egui::Ui,
    title: Option<&str>,
    selected: &mut T,
) -> bool {
    let choices = T::all()
        .into_iter()
        .map(|&choice| (choice, choice.as_str()));
    choice_buttons(ui, title, choices, selected)
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
