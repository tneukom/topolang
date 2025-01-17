use crate::{
    brush::Brush,
    coordinate_frame::CoordinateFrames,
    field::RgbaField,
    history::{Snapshot, SnapshotCause},
    interpreter::Interpreter,
    material::Material,
    math::{point::Point, rect::Rect, rgba8::Pico8Palette},
    painting::view_painter::{DrawView, ViewPainter},
    pixmap::MaterialMap,
    utils::{IntoT, ReflectEnum},
    view::{EditMode, View, ViewInput, ViewSettings},
    widgets::{brush_chooser, FileChooser},
    world::World,
};
use anyhow::Context;
use data_encoding::BASE64;
use egui::Widget;
use glow::HasContext;
use image::{
    codecs::gif::{GifEncoder, Repeat},
    ExtendedColorType,
};
use instant::Instant;
use log::{info, warn};
use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
};

pub struct GifRecorder {
    start: Option<Rc<Snapshot>>,
    stop: Option<Rc<Snapshot>>,
}

impl GifRecorder {
    pub fn new() -> Self {
        Self {
            start: None,
            stop: None,
        }
    }

    /// Try to find a path from start to stop or from stop to start
    fn history_path(&self) -> Option<Vec<&Rc<Snapshot>>> {
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
                .to_rgba8_field(Material::TRANSPARENT);

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
        let rgba_field = self.material_map.to_rgba8_field(Material::TRANSPARENT);
        let png = rgba_field.to_png().unwrap();
        let base64_png = BASE64.encode(&png);
        format!("{}{}", Self::DATA_URL_HEADER, base64_png)
    }
}

pub struct EguiApp {
    view_painter: Arc<Mutex<ViewPainter>>,
    start_time: Instant,
    pub view_settings: ViewSettings,

    gl: Arc<glow::Context>,
    view: View,

    view_input: ViewInput,

    new_size: Point<i64>,
    file_name: String,
    // current_folder: PathBuf,
    run: bool,
    interpreter: Interpreter,

    // stabilize: bool,
    // stabilize_count: i64,
    file_chooser: FileChooser,

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

    pub unsafe fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let view_painter = ViewPainter::new(cc.gl.as_ref().unwrap().clone());
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

        let gl = cc.gl.clone().unwrap();

        let view_settings = ViewSettings {
            edit_mode: EditMode::Brush,
            brush: Brush::default(),
        };

        let (channel_sender, channel_receiver) = mpsc::sync_channel(1);

        let saves_path = PathBuf::from("resources/saves")
            .canonicalize()
            .expect("Failed to canonicalize saves path");

        Self {
            view_painter: Arc::new(Mutex::new(view_painter)),
            start_time,
            view,
            view_settings,
            gl,
            new_size: Point(512, 512),
            file_name: "".to_string(),
            run: false,
            view_input: ViewInput::EMPTY,
            file_chooser: FileChooser::new(saves_path),
            interpreter: Interpreter::new(),
            gif_recorder: GifRecorder::new(),
            clipboard: None,
            channel_sender,
            channel_receiver,
            reset_camera_requested: true,
        }
    }

    pub fn edit_mode_icon(edit_mode: EditMode) -> egui::ImageSource<'static> {
        match edit_mode {
            EditMode::Brush => egui::include_image!("icons/brush.png"),
            EditMode::Eraser => egui::include_image!("icons/eraser.png"),
            EditMode::Fill => egui::include_image!("icons/fill.png"),
            EditMode::SelectRect => egui::include_image!("icons/select_rect.png"),
            EditMode::PickColor => egui::include_image!("icons/pick.png"),
            EditMode::SelectWand => egui::include_image!("icons/wand.png"),
        }
    }

    pub fn view_ui(&mut self, ui: &mut egui::Ui) {
        // Mouse world position
        // ui.label(format!(
        //     "snapped mouse: {:?}",
        //     self.view_input.world_snapped
        // ));

        if ui.button("Reset camera").clicked() {
            self.reset_camera_requested = true;
        }

        // Edit mode choices
        ui.horizontal(|ui| {
            for mode in EditMode::ALL {
                let icon = Self::edit_mode_icon(mode);

                // The behavior of Egui when clicking a button and moving the mouse is a bit weird.
                // If a native Windows button is pressed down, the mouse moved while still inside
                // the button and then released, it counts as a click.
                // Egui aborts the clicked state if the mouse is moved too much. So we also consider
                // dragging and check if the pointer is still over the button.
                let button = egui::widgets::ImageButton::new(icon)
                    .selected(mode == self.view_settings.edit_mode)
                    .sense(egui::Sense::click_and_drag());
                let response = ui.add_sized([32.0, 32.0], button);

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
                    .to_rgba8_field(Material::TRANSPARENT);
                if let Err(err) = bitmap.save(path) {
                    warn!("Failed to save with error {err}");
                }
            }
        }
    }

    pub fn load_save_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                let bounds = Rect::low_size(Point(0, 0), self.new_size);
                self.view = View::empty(bounds);
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
                .to_rgba8_field(Material::TRANSPARENT);
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

    pub fn document_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let bounds = self.view.world.bounding_rect();
            let size = bounds.size();
            ui.label(format!("Size: {} x {}", size.x, size.y));
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Set size:");
            for size in [16, 32, 64, 128, 256, 512, 1024, 2048] {
                if ui.button(format!("{size}")).clicked() {
                    let bounds = Rect::low_size(Point::ZERO, Point(size, size));
                    self.view.resize(bounds);
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
            self.view.history.add_snapshot(
                self.view.world.material_map().clone(),
                self.view.selection.clone(),
                SnapshotCause::Step,
            );
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
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();

        egui::scroll_area::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(200.0)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
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
            self.view.selection = self.view.history.head.selection().clone();
        }
    }

    pub fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        self.view_ui(ui);
        ui.separator();

        self.copy_paste_ui(ui);
        ui.separator();

        ui.label("Brush");

        brush_chooser(ui, &mut self.view_settings.brush);

        ui.label("History");
        self.history_ui(ui);
        ui.separator();

        ui.label("Run");
        self.run_ui(ui);
        ui.separator();

        ui.label("Gif");
        self.gif_ui(ui);
        ui.separator();

        ui.label("Document");
        self.document_ui(ui);
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
        self.new_size = rgba_field.size();
        let world = rgba_field.intot::<MaterialMap>().into();
        self.view = View::new(world);
        self.reset_camera_requested = true;
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

    fn central_panel(&mut self, ui: &mut egui::Ui) {
        let input = ui.ctx().input(|input| input.clone());
        let window_size: Point<f64> = input.screen_rect.size().into();

        let size = ui.available_size_before_wrap();
        let (egui_rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

        let viewport: Rect<f64> = egui_rect.into();

        let frames = CoordinateFrames {
            window_size,
            viewport,
        };

        // `ctx.pointer_interact_pos()` is None if mouse is outside the window
        if let Some(egui_mouse) = ui.ctx().pointer_interact_pos() {
            let window_mouse = Point::new(egui_mouse.x as f64, egui_mouse.y as f64);
            let view_mouse = frames.window_to_view() * window_mouse;

            // Scroll captured if the mouse pointer is over view, even if it doesn't have focus.
            let scroll_delta = if response.contains_pointer() {
                input.smooth_scroll_delta.y as f64 / 50.0
            } else {
                0.0
            };

            let wants_keyboard = ui.ctx().wants_keyboard_input();
            let hovered = response.hovered();

            // TODO: wants_keyboard is false when cursor is in textbox and escape is pressed, so a
            //   selection is cancelled. It should not be cancelled!
            self.view_input = ViewInput {
                frames,
                view_mouse,

                world_mouse: Point::ZERO,
                world_snapped: Point::ZERO,

                left_mouse_down: hovered && input.pointer.button_down(egui::PointerButton::Primary),
                middle_mouse_down: hovered
                    && input.pointer.button_down(egui::PointerButton::Middle),

                escape_pressed: !wants_keyboard && input.key_pressed(egui::Key::Escape),
                ctrl_down: !wants_keyboard && input.modifiers.ctrl,
                delete_pressed: !wants_keyboard && input.key_pressed(egui::Key::Delete),

                mouse_wheel: scroll_delta,
            };
        }

        // Reset camera requested, for example from loading World in Self::new
        if self.reset_camera_requested {
            let view_rect = Rect::low_size(Point::ZERO, frames.viewport.size());
            self.view.center_camera(view_rect);
            self.reset_camera_requested = false;
        }

        let time = self.time();
        let draw_view = DrawView::from_view(
            &self.view,
            &self.view_settings,
            &self.view_input,
            frames,
            time,
        );

        let view_painter = self.view_painter.clone();

        let cb = egui_glow::CallbackFn::new(move |_info, painter| {
            let gl = painter.gl().clone();
            let mut view_painter = view_painter.lock().unwrap();

            unsafe {
                // Print the active opengl viewport for debugging
                // let mut gl_viewport = [0; 4];
                // unsafe {
                //     gl.get_parameter_i32_slice(glow::VIEWPORT, &mut gl_viewport);
                // }
                // println!(
                //     "glViewport(x: {}, y: {}, width: {}, height: {})",
                //     gl_viewport[0], gl_viewport[1], gl_viewport[2], gl_viewport[3]
                // );

                // gl.scissor(
                //     viewport.left() as i32,
                //     (window_size.y - viewport.bottom()) as i32,
                //     viewport.width() as i32,
                //     viewport.height() as i32,
                // );

                // gl.enable(glow::SCISSOR_TEST);
                // gl.clear_color(1.0f32, 0.0f32, 0.0f32, 1.0f32);
                // gl.clear(glow::COLOR_BUFFER_BIT);

                gl.disable(glow::BLEND);
                gl.disable(glow::SCISSOR_TEST);
                gl.disable(glow::CULL_FACE);
                gl.disable(glow::DEPTH_TEST);
                // self.gl.enable(glow::FRAMEBUFFER_SRGB);

                view_painter.draw_view(&draw_view);
            }
        });

        let callback = egui::PaintCallback {
            rect: egui_rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
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
                egui::Event::Paste(paste) => {
                    if let Some(clipboard) = Clipboard::decode(&paste) {
                        self.view
                            .clipboard_paste(&self.view_input, clipboard.material_map);
                    }
                }
                egui::Event::Copy => {
                    self.clipboard_copy(ctx);
                }
                egui::Event::Cut => {
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

        egui::SidePanel::left("left_panel")
            .show(ctx, |ui| {
                self.side_panel_ui(ui);
                ui.allocate_space(ui.available_size());
            })
            .response
            .rect;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.top_ui(ui);
            // ui.allocate_space(ui.available_size());
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.central_panel(ui);
        });

        self.view
            .handle_input(&mut self.view_input, &mut self.view_settings);

        let cursor_icon = if self.view_settings.edit_mode == EditMode::Brush {
            egui::CursorIcon::Default
        } else if self.view.ui_state.is_idle() && self.view.is_hovering_selection(&self.view_input)
        {
            egui::CursorIcon::Move
        } else {
            egui::CursorIcon::Default
        };
        ctx.set_cursor_icon(cursor_icon);

        #[cfg(feature = "force_120hz")]
        ctx.request_repaint_after_secs(1.0f32 / 120.0f32);

        #[cfg(not(feature = "force_120hz"))]
        ctx.request_repaint();
    }
}
