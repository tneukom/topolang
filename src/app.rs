use anyhow::Context;
use data_encoding::BASE64;
use std::{
    collections::HashMap,
    fs,
    fs::File,
    hash::Hash,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{mpsc, Arc},
};

use egui::{
    load::SizedTexture, scroll_area::ScrollBarVisibility, style::ScrollStyle, CursorIcon, Event,
    Sense, TextureOptions, Widget,
};
use glow::HasContext;
use image::{
    codecs::gif::{GifEncoder, Repeat},
    ExtendedColorType,
};
use instant::Instant;
use log::{info, warn};

use crate::{
    brush::Brush,
    coordinate_frame::CoordinateFrames,
    field::RgbaField,
    history::{Snapshot, SnapshotCause},
    interpreter::Interpreter,
    material::Material,
    math::{point::Point, rect::Rect, rgba8::Pico8Palette},
    painting::view_painter::ViewPainter,
    pixmap::MaterialMap,
    utils::{IntoT, ReflectEnum},
    view::{EditMode, View, ViewButton, ViewInput, ViewSettings},
    widgets::{BrushChooser, ColorChooser, FileChooser},
    world::World,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

impl MouseButton {
    const VALUES: [MouseButton; 3] = [MouseButton::Left, MouseButton::Middle, MouseButton::Right];
}

impl From<egui::PointerButton> for MouseButton {
    fn from(egui_button: egui::PointerButton) -> Self {
        match egui_button {
            egui::PointerButton::Primary => MouseButton::Left,
            egui::PointerButton::Secondary => MouseButton::Right,
            egui::PointerButton::Middle => MouseButton::Middle,
            _ => unimplemented!("Not handling any other mouse buttons"),
        }
    }
}

impl From<MouseButton> for egui::PointerButton {
    fn from(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => egui::PointerButton::Primary,
            MouseButton::Right => egui::PointerButton::Secondary,
            MouseButton::Middle => egui::PointerButton::Middle,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MouseButtonState {
    Up,
    Pressed,
    Down,
    Rejected,
}

impl MouseButtonState {
    pub fn is_down(self) -> bool {
        self == Self::Down || self == Self::Pressed
    }
}

pub struct GifRecorder {
    start: Option<Rc<Snapshot<Material>>>,
    stop: Option<Rc<Snapshot<Material>>>,
}

impl GifRecorder {
    pub fn new() -> Self {
        Self {
            start: None,
            stop: None,
        }
    }

    /// Try to find a path from start to stop or from stop to start
    fn history_path(&self) -> Option<Vec<&Rc<Snapshot<Material>>>> {
        let start = self.start.as_ref()?;
        let stop = self.stop.as_ref()?;
        if let Some(path) = stop.path_to(start) {
            return Some(path);
        }
        if let Some(path) = start.path_to(stop) {
            return Some(path);
        }
        None
    }

    pub fn export(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let snapshot_path = self.history_path().context("No path")?;

        let file = File::create(path)?;
        let mut gif_encoder = GifEncoder::new_with_speed(file, 10);
        gif_encoder.set_repeat(Repeat::Infinite).unwrap();

        for snapshot in snapshot_path {
            // TODO: Offset world pixmap by bounds.low()
            // TODO: Paint over white background to remove transparency or use apng instead
            let image = snapshot
                .material_map()
                .to_field(Material::TRANSPARENT)
                .into_rgba();

            gif_encoder.encode(
                image.as_raw(),
                image.width() as u32,
                image.height() as u32,
                ExtendedColorType::Rgba8,
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Clipboard {
    pub material_map: MaterialMap,
}

impl Clipboard {
    const DATA_URL_HEADER: &'static str = "data:image/png;base64,";

    pub fn new(material_map: MaterialMap) -> Self {
        Self { material_map }
    }

    pub fn decode(encoded: &str) -> Option<Self> {
        // Strip DATA_URL_HEADER
        if encoded.len() <= Self::DATA_URL_HEADER.len() {
            return None;
        }
        let payload = &encoded[Self::DATA_URL_HEADER.len()..];
        let png = BASE64.decode(payload.as_bytes()).ok()?;
        let rgba_field = RgbaField::load_from_memory(&png).ok()?;
        let material_map = rgba_field.into_material().into();
        Some(Self { material_map })
    }

    pub fn encode(&self) -> String {
        let rgba_field = self
            .material_map
            .to_field(Material::TRANSPARENT)
            .into_rgba();
        let png = rgba_field.to_png().unwrap();
        let base64_png = BASE64.encode(&png);
        format!("{}{}", Self::DATA_URL_HEADER, base64_png)
    }
}

pub struct EguiApp {
    view_painter: ViewPainter,
    start_time: Instant,
    pub view_settings: ViewSettings,

    view_rect: Rect<i64>,

    gl: Arc<glow::Context>,
    view: View,
    mouse_button_states: HashMap<MouseButton, MouseButtonState>,

    view_input: ViewInput,

    new_size: Point<i64>,
    file_name: String,
    // current_folder: PathBuf,
    run: bool,
    interpreter: Interpreter,

    // stabilize: bool,
    // stabilize_count: i64,
    edit_mode_icons: HashMap<EditMode, egui::TextureHandle>,

    file_chooser: FileChooser,
    brush_chooser: BrushChooser,

    gif_recorder: GifRecorder,

    channel_receiver: mpsc::Receiver<Vec<u8>>,
    channel_sender: mpsc::SyncSender<Vec<u8>>,

    clipboard: Option<Clipboard>,

    /// When creating App we already load the world but cannot reset the camera because we don't
    /// have the proper view_rect
    reset_camera_requested: bool,
}

impl EguiApp {
    fn time(&self) -> f64 {
        (Instant::now() - self.start_time).as_secs_f64()
    }

    fn key_button(ctx: &egui::Context, key: egui::Key) -> ViewButton {
        if ctx.wants_keyboard_input() {
            ViewButton::up()
        } else {
            ViewButton {
                is_down: ctx.input(|input| input.key_down(key)),
                is_pressed: ctx.input(|input| input.key_pressed(key)),
            }
        }
    }

    fn ctrl_key(ctx: &egui::Context) -> ViewButton {
        ViewButton {
            is_down: ctx.input(|input| input.modifiers.ctrl),
            // TODO: Not great, is there no function to check if a modifier was pressed?
            is_pressed: false,
        }
    }

    fn shift_key(ctx: &egui::Context) -> ViewButton {
        ViewButton {
            is_down: ctx.input(|input| input.modifiers.shift),
            is_pressed: false,
        }
    }

    fn mouse_button(&self, mouse_button: MouseButton) -> ViewButton {
        let state = self.mouse_button_states[&mouse_button];
        ViewButton {
            is_down: state.is_down(),
            is_pressed: state == MouseButtonState::Pressed,
        }
    }

    pub unsafe fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let scene_painter = ViewPainter::new(cc.gl.as_ref().unwrap().clone());
        let start_time = Instant::now();

        // Load topology from file
        // TODO: Should be fetched instead of included
        let world_image_bytes = include_bytes!("../resources/saves/turing.png");
        // let world_bitmap = Bitmap::transparent(512, 512);
        let world_color_map = RgbaField::load_from_memory(world_image_bytes)
            .unwrap()
            .into_material()
            .into();
        let world = World::from_material_map(world_color_map);
        // let world = Topology::from_bitmap_path("test_resources/compiler/gate/world.png").unwrap();
        let view = View::new(world);

        let mouse_button_states: HashMap<_, _> = MouseButton::VALUES
            .into_iter()
            .map(|button| (button, MouseButtonState::Up))
            .collect();

        let gl = cc.gl.clone().unwrap();

        let load_egui_icon = |name: &str, bytes: &[u8]| {
            let bitmap = RgbaField::load_from_memory(bytes).unwrap();
            let raw: &[u8] = &bitmap.as_raw();
            let size = [bitmap.width(), bitmap.height()];
            let egui_image =
                egui::ColorImage::from_rgba_unmultiplied([size[0] as usize, size[1] as usize], raw);

            cc.egui_ctx
                .load_texture(name, egui_image, TextureOptions::LINEAR)
        };

        let edit_mode_icons = HashMap::from([
            (
                EditMode::Brush,
                load_egui_icon("icons/brush.png", include_bytes!("icons/brush.png")),
            ),
            (
                EditMode::Eraser,
                load_egui_icon("icons/eraser.png", include_bytes!("icons/eraser.png")),
            ),
            (
                EditMode::Fill,
                load_egui_icon("icons/fill.png", include_bytes!("icons/fill.png")),
            ),
            (
                EditMode::SelectRect,
                load_egui_icon(
                    "icons/select_rect.png",
                    include_bytes!("icons/select_rect.png"),
                ),
            ),
        ]);

        let view_settings = ViewSettings {
            edit_mode: EditMode::Brush,
            brush: Brush::default(),
        };

        let (channel_sender, channel_receiver) = mpsc::sync_channel(1);

        let saves_path = PathBuf::from("resources/saves")
            .canonicalize()
            .expect("Failed to canonicalize saves path");

        Self {
            view_painter: scene_painter,
            start_time,
            view,
            view_settings,
            view_rect: Rect::low_size([0, 0], [1, 1]),
            mouse_button_states,
            gl,
            new_size: Point(512, 512),
            file_name: "".to_string(),
            run: false,
            // stabilize: false,
            // stabilize_count: 0,
            // current_folder: PathBuf::from("resources/saves"),
            view_input: ViewInput::EMPTY,
            edit_mode_icons,
            file_chooser: FileChooser::new(saves_path),
            brush_chooser: BrushChooser::new(ColorChooser::default()),
            interpreter: Interpreter::new(),
            gif_recorder: GifRecorder::new(),
            clipboard: None,
            channel_sender,
            channel_receiver,
            reset_camera_requested: true,
        }
    }

    fn view_size(ctx: &egui::Context) -> Point<i64> {
        let egui_view_size = ctx.input(|input| input.screen_rect.size());
        Point::new(egui_view_size.x as i64, egui_view_size.y as i64)
    }

    fn frames(ctx: &egui::Context) -> CoordinateFrames {
        let view_size = Self::view_size(ctx);
        CoordinateFrames::new(view_size.x as i64, view_size.y as i64)
    }

    pub fn view_input_from(&mut self, ctx: &egui::Context) -> ViewInput {
        for button in MouseButton::VALUES {
            let egui_button = egui::PointerButton::from(button);
            let state = self.mouse_button_states[&button];
            let egui_button_down = ctx.input(|input| input.pointer.button_down(egui_button));

            let new_state = if egui_button_down {
                match state {
                    MouseButtonState::Up => {
                        if ctx.is_pointer_over_area() {
                            MouseButtonState::Rejected
                        } else {
                            MouseButtonState::Pressed
                        }
                    }
                    MouseButtonState::Pressed => MouseButtonState::Down,
                    other => other,
                }
            } else {
                MouseButtonState::Up
            };
            self.mouse_button_states.insert(button, new_state);
        }

        let window_mouse = match ctx.pointer_latest_pos() {
            Some(egui_mouse) => Point::new(egui_mouse.x as f64, egui_mouse.y as f64),
            None => Point::ZERO,
        };

        let frames = Self::frames(ctx);
        let view_mouse = frames.window_to_view() * window_mouse;

        let scroll_delta = if ctx.wants_pointer_input() {
            0.0
        } else {
            ctx.input(|input| input.smooth_scroll_delta.y as f64 / 50.0)
        };

        let input = ViewInput {
            view_size: Self::view_size(ctx),
            view_mouse,

            world_mouse: Point::ZERO,
            world_snapped: Point::ZERO,

            left_mouse: self.mouse_button(MouseButton::Left),
            middle_mouse: self.mouse_button(MouseButton::Middle),
            right_mouse: self.mouse_button(MouseButton::Right),

            shift_key: Self::shift_key(ctx),
            ctrl_key: Self::ctrl_key(ctx),
            delete_key: Self::key_button(ctx, egui::Key::Delete),
            enter_key: Self::key_button(ctx, egui::Key::Enter),
            escape_key: Self::key_button(ctx, egui::Key::Escape),

            mouse_wheel: scroll_delta,
        };

        input
    }

    pub fn reset_camera(&mut self) {
        self.view.center_camera(self.view_rect.cwise_cast());
    }

    pub fn view_ui(&mut self, ui: &mut egui::Ui) {
        // Mouse world position
        ui.label(format!(
            "snapped mouse: {:?}",
            self.view_input.world_snapped
        ));

        if ui.button("Reset camera").clicked() {
            self.reset_camera();
        }

        // Edit mode choices
        ui.horizontal(|ui| {
            for mode in EditMode::ALL {
                let texture = self.edit_mode_icons.get(&mode).unwrap();
                //let texture_id = egui::TextureId::User(texture.id.0.get() as u64);
                let texture_id = texture.id();

                // The behavior of Egui when clicking a button and moving the mouse is a bit weird.
                // If a native Windows button is pressed down, the mouse moved while still inside
                // the button and then released, it counts as a click.
                // Egui aborts the clicked state if the mouse is moved too much. So we also consider
                // dragging and check if the pointer is still over the button.
                let button =
                    egui::widgets::ImageButton::new(SizedTexture::new(texture_id, (20.0, 20.0)))
                        .selected(mode == self.view_settings.edit_mode)
                        .sense(Sense::click_and_drag());
                let response = ui.add(button);

                if response.clicked() || response.drag_stopped() && response.hover_pos().is_some() {
                    self.view_settings.edit_mode = mode;
                }
            }
        });
    }

    pub fn clipboard_put(&mut self, ctx: &egui::Context, material_map: MaterialMap) {
        let clipboard = Clipboard::new(material_map);
        let encoded = clipboard.encode();
        ctx.output_mut(|output| output.copied_text = encoded);
        self.clipboard = Some(clipboard);
    }

    pub fn clipboard_copy(&mut self, ctx: &egui::Context) {
        if let Some(material_map) = self.view.clipboard_copy() {
            self.clipboard_put(ctx, material_map);
        }
    }

    pub fn clipboard_cut(&mut self, ctx: &egui::Context) {
        if let Some(material_map) = self.view.clipboard_cut() {
            self.clipboard_put(ctx, material_map);
        }
    }

    // pub fn clipboard_paste(&mut self) {
    //     let Some(clipboard) = &self.clipboard else {
    //         return;
    //     };
    //
    //     self.view.clipboard_paste(clipboard.clone());
    // }

    pub fn copy_paste_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Copy").clicked() {
                self.clipboard_copy(ui.ctx());
            }

            if ui.button("Cut").clicked() {
                self.clipboard_cut(ui.ctx());
            }

            if ui.button("Paste").clicked() {
                if let Some(clipboard) = &self.clipboard {
                    self.view
                        .clipboard_paste(&self.view_input, clipboard.material_map.clone());
                }
                // TODO: Use ViewportCommand::RequestPaste
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open_save_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("Open File").clicked() {
            let channel_sender = self.channel_sender.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(file) = rfd::AsyncFileDialog::new()
                    .add_filter("png", &["png"])
                    .pick_file()
                    .await
                {
                    let content = file.read().await;
                    channel_sender.send(content).unwrap();
                }
            });
        }

        if ui.button("Save File").clicked() {
            let task = rfd::AsyncFileDialog::new().save_file();
            let color_map = self.view.world.material_map();
            let bitmap = color_map.to_bitmap();
            match bitmap.to_png() {
                Ok(file_content) => {
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(file) = rfd::AsyncFileDialog::new()
                            .set_file_name("world.png")
                            .add_filter("png", &["png"])
                            .save_file()
                            .await
                        {
                            if let Err(result) = file.write(&file_content).await {
                                warn!("Failed to save png");
                            }
                        }
                    });
                }
                Err(err) => {
                    warn!("Failed to convert bitmap to png with error {err}");
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_save_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("Open File").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("png", &["png"])
                .pick_file()
            {
                self.load_from_path(path);
            }
        }

        if ui.button("Save File").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("png", &["png"])
                .save_file()
            {
                warn!("Saving to path {:?}", path.to_str());
                let bitmap = self
                    .view
                    .world
                    .material_map()
                    .to_field(Material::TRANSPARENT)
                    .into_rgba();
                if let Err(err) = bitmap.save(path) {
                    warn!("Failed to save with error {err}");
                }
            }
        }
    }

    pub fn load_save_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Width:");
            ui.add(
                egui::DragValue::new(&mut self.new_size.x)
                    .clamp_range(0..=1024)
                    .speed(2.0),
            );
            ui.label("Height:");
            ui.add(
                egui::DragValue::new(&mut self.new_size.y)
                    .clamp_range(0..=1024)
                    .speed(2.0),
            );
        });

        ui.horizontal(|ui| {
            let bounds = Rect::low_size([0, 0], self.new_size);

            if ui.button("New").clicked() {
                self.view = View::empty(bounds);
            }

            if ui.button("Resize").clicked() {
                self.view.resize(bounds);
            }
        });

        let path = self.file_chooser.show(ui);

        if ui.button("Load").clicked() {
            println!("Loading file {:?} in folder", &path);
            self.load_from_path(&path);
            ui.close_menu();
        }

        if ui.button("Save").clicked() {
            println!("Saving edit scene to {}", self.file_name);

            let bitmap = self
                .view
                .world
                .material_map()
                .to_field(Material::TRANSPARENT)
                .into_rgba();
            match bitmap.save(&path) {
                Ok(_) => println!("Saved {path:?}"),
                Err(err) => println!("Failed to save {path:?} with error {err}"),
            }

            ui.close_menu();
        }
    }

    pub fn gif_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Set Start").clicked() {
                self.gif_recorder.start = Some(self.view.history.head.clone());
            }

            if ui.button("Set stop").clicked() {
                self.gif_recorder.stop = Some(self.view.history.head.clone());
            }

            if ui.button("Export").clicked() {
                if let Err(err) = self.gif_recorder.export("movie.gif") {
                    warn!("Failed to export gif with {err}");
                }
            }
        });
    }

    pub fn run_ui(&mut self, ui: &mut egui::Ui) {
        // Step and run
        let mut do_step: bool = false;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.run, egui::Button::new("Step"))
                .clicked()
            {
                do_step = true;
            }

            if egui::Button::new("Run").selected(self.run).ui(ui).clicked() {
                self.run = !self.run;
            }
        });

        if self.run {
            do_step = true;
        }

        if do_step {
            self.interpreter.step(&mut self.view.world);
            self.view
                .history
                .add_snapshot(self.view.world.material_map().clone(), SnapshotCause::Step);
        }
    }

    /// If the user reverts to a snapshot in the history that snapshot is returned.
    pub fn history_ui(&mut self, ui: &mut egui::Ui) {
        let current_head = self.view.history.head.clone();
        let history = &mut self.view.history;
        // List of snapshots with cause
        let path = history.active.path_to(&history.root).unwrap();

        // Floating scroll bars, see
        // https://github.com/emilk/egui/pull/3539
        ui.style_mut().spacing.scroll = ScrollStyle::solid();

        egui::scroll_area::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(200.0)
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                for snapshot in path.into_iter() {
                    ui.horizontal(|ui| {
                        let mut btn = egui::Button::new(snapshot.cause().as_str());
                        if Rc::ptr_eq(snapshot, &history.head) {
                            btn = btn.fill(Pico8Palette::ORANGE);
                        }
                        if btn.ui(ui).clicked() {
                            history.head = snapshot.clone();
                        }

                        // Add gif start/stop labels after button
                        if let Some(gif_start) = &self.gif_recorder.start {
                            if Rc::ptr_eq(gif_start, snapshot) {
                                ui.label("Gif start");
                            }
                        };
                        if let Some(gif_stop) = &self.gif_recorder.stop {
                            if Rc::ptr_eq(gif_stop, snapshot) {
                                ui.label("Gif stop");
                            }
                        };
                    });
                }
            });

        // Undo, Redo buttons
        ui.horizontal(|ui| {
            if ui.button("Undo").clicked() {
                history.undo();
            }
            if ui.button("Redo").clicked() {
                history.redo();
            }
        });

        // If head has changed, update world
        if !Rc::ptr_eq(&self.view.history.head, &current_head) {
            self.view.world =
                World::from_material_map(self.view.history.head.material_map().clone());
        }
    }

    pub fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        self.view_ui(ui);
        ui.separator();

        self.copy_paste_ui(ui);
        ui.separator();

        ui.label("Brush");
        self.brush_chooser.show(ui);
        self.view_settings.brush = self.brush_chooser.brush();

        ui.label("History");
        self.history_ui(ui);
        ui.separator();

        ui.label("Run");
        self.run_ui(ui);
        ui.separator();

        ui.label("Gif");
        self.gif_ui(ui);
    }

    pub fn top_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                self.load_save_ui(ui);
            });
            ui.menu_button("File2", |ui| {
                self.open_save_ui(ui);
            })
        });
    }

    fn load_file(&mut self, content: &[u8]) {
        info!("Loading a file!");
        let Ok(rgba_field) = RgbaField::load_from_memory(content) else {
            warn!("Failed to load png file!");
            return;
        };
        self.new_size = rgba_field.bounds().size();
        let world = rgba_field.intot::<MaterialMap>().into();
        self.view = View::new(world);
        self.reset_camera();
    }

    fn load_from_path(&mut self, path: impl AsRef<Path>) {
        warn!("Loading from path {:?}", path.as_ref().to_str());
        let content = match fs::read(path) {
            Ok(content) => content,
            Err(err) => {
                warn!("Failed to load file with error {}", err);
                return;
            }
        };
        self.load_file(&content);
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // let mut style = ctx.style().deref().clone();
        // style.visuals.dark_mode = false;
        // ctx.set_style(style);

        // Check if any files have finished loading
        #[cfg(target_arch = "wasm32")]
        if let Ok(content) = self.channel_receiver.try_recv() {
            self.load_file(&content);
        }

        // We cannot lock input while called clipboard_copy, clipboard_cut, ...
        // TODO: Only clone Paste, Copy, Cut events
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            match event {
                Event::Paste(paste) => {
                    if let Some(clipboard) = Clipboard::decode(&paste) {
                        self.view
                            .clipboard_paste(&self.view_input, clipboard.material_map);
                    }
                }
                Event::Copy => {
                    self.clipboard_copy(ctx);
                }
                Event::Cut => {
                    self.clipboard_cut(ctx);
                }
                _ => {}
            }
        }

        ctx.input(|input| {
            if let Some(file) = input.raw.dropped_files.last().cloned() {
                warn!(
                    "File dropped with name {} and path {:?}",
                    file.name, file.path
                );
                if let Some(path) = file.path {
                    self.load_from_path(path);
                }
            }
        });

        let visual = egui::Visuals::light();
        // visual.window_shadow = epaint::Shadow::NONE;
        ctx.set_visuals(visual);

        let side_panel_rect = egui::SidePanel::left("left_panel")
            .show(ctx, |ui| {
                self.side_panel_ui(ui);
                ui.allocate_space(ui.available_size());
            })
            .response
            .rect;

        let _top_panel_rect = egui::TopBottomPanel::top("top_panel")
            .show(ctx, |ui| {
                self.top_ui(ui);
                // ui.allocate_space(ui.available_size());
            })
            .response
            .rect;

        let view_size = Self::view_size(ctx);
        self.view_rect = Rect::low_high([side_panel_rect.right() as i64, 0], view_size);

        // Reset camera requested, for example from loading World in Self::new
        if self.reset_camera_requested {
            self.reset_camera();
            self.reset_camera_requested = false;
        }

        // view_input after the UI has been drawn, so we know if the cursor is
        // over an element.
        self.view_input = self.view_input_from(ctx);
        let time = self.time();

        self.view
            .handle_input(&mut self.view_input, &self.view_settings);
        let frames = self.view_input.frames();
        // let preview = self.view.preview(&self.view_settings);

        let cursor_icon =
            if self.view.ui_state.is_idle() && self.view.is_hovering_selection(&self.view_input) {
                CursorIcon::Move
            } else {
                CursorIcon::Default
            };
        ctx.set_cursor_icon(cursor_icon);

        unsafe {
            self.gl.disable(glow::BLEND);
            self.gl.disable(glow::SCISSOR_TEST);
            self.gl.disable(glow::CULL_FACE);
            self.gl.disable(glow::DEPTH_TEST);
            // self.gl.enable(glow::FRAMEBUFFER_SRGB);

            self.gl
                .clear_color(89.0 / 255.0, 124.0 / 255.0, 149.0 / 255.0, 1.0);
            // self.gl.clear_color(1.0, 1.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            self.view_painter.draw_view(&self.view, &frames, time);
        }

        ctx.request_repaint();
    }
}
