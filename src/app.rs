use crate::{
    brush::Brush,
    compiler::{CompileError, Compiler},
    coordinate_frame::CoordinateFrames,
    field::RgbaField,
    gif_recorder::GifRecorder,
    history::SnapshotCause,
    interpreter::{Interpreter, InterpreterError},
    material::Material,
    material_effects::material_map_effects,
    math::{point::Point, rect::Rect, rgba8::Rgba8},
    painting::view_painter::{DrawView, ViewPainter},
    pixmap::MaterialMap,
    rule::CanvasInput,
    utils::ReflectEnum,
    view::{EditMode, Selection, View, ViewInput, ViewSettings},
    widgets::{
        FileChooser, brush_chooser, enum_choice_buttons, prefab_picker, styled_button, styled_space,
    },
    world::World,
};
use egui::AtomExt;
use glow::HasContext;
use itertools::Itertools;
use log::{info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex, mpsc},
};
use web_time::Instant;

#[derive(Debug, Clone)]
pub struct Clipboard {
    pub material_map: MaterialMap,
}

impl Clipboard {
    pub fn new(material_map: MaterialMap) -> Self {
        Self { material_map }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunSpeed {
    Hz1,
    Hz2,
    Hz5,
    Hz10,
    Hz30,
}

impl RunSpeed {
    const ALL: [Self; 5] = [Self::Hz1, Self::Hz2, Self::Hz5, Self::Hz10, Self::Hz30];

    pub fn frames_per_tick(self) -> usize {
        match self {
            Self::Hz1 => 60,
            Self::Hz2 => 30,
            Self::Hz5 => 12,
            Self::Hz10 => 6,
            Self::Hz30 => 2,
        }
    }
}

impl ReflectEnum for RunSpeed {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Hz1 => "1 Hz",
            Self::Hz2 => "2 Hz",
            Self::Hz5 => "3 Hz",
            Self::Hz10 => "10 Hz",
            Self::Hz30 => "30 Hz",
        }
    }
}

pub struct EguiApp {
    view_painter: Arc<Mutex<ViewPainter>>,
    start_time: Instant,
    pub view_settings: ViewSettings,

    gl: Arc<glow::Context>,
    view: View,

    view_input: ViewInput,
    canvas_input: CanvasInput,

    new_size: Point<i64>,
    file_name: String,
    // current_folder: PathBuf,
    compiler: Compiler,
    compile_error: Option<CompileError>,
    interpreter: Option<Interpreter>,
    run_mode: RunMode,
    run_speed: RunSpeed,

    // stabilize: bool,
    // stabilize_count: i64,
    #[cfg(not(target_arch = "wasm32"))]
    file_chooser: FileChooser,

    gif_recorder: Option<GifRecorder>,

    channel_receiver: mpsc::Receiver<Vec<u8>>,
    channel_sender: mpsc::SyncSender<Vec<u8>>,

    clipboard: Option<Clipboard>,

    /// When creating App we already load the world but cannot reset the camera because we don't
    /// have the proper view_rect
    reset_camera_requested: bool,

    i_frame: usize,
    modifications_during_tick: usize,
    woken_up_during_tick: usize,

    #[cfg(feature = "link_ui")]
    link: String,
}

impl EguiApp {
    fn time(&self) -> f64 {
        (Instant::now() - self.start_time).as_secs_f64()
    }

    pub unsafe fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // let gl = cc.gl.as_ref().map(|arc| arc.as_ref());
        let gl_arc = cc.gl.clone().unwrap();
        let gl = gl_arc.as_ref();
        let view_painter = ViewPainter::new(gl);
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
            locked: false,
            edit_mode: EditMode::Brush,
            brush: Brush::default(),
        };

        let (channel_sender, channel_receiver) = mpsc::sync_channel(1);

        Self {
            view_painter: Arc::new(Mutex::new(view_painter)),
            start_time,
            view,
            view_settings,
            gl,
            new_size: Point(512, 512),
            file_name: "".to_string(),
            run_mode: RunMode::Paused,
            run_speed: RunSpeed::Hz30,
            view_input: ViewInput::EMPTY,
            canvas_input: CanvasInput::default(),
            #[cfg(not(target_arch = "wasm32"))]
            file_chooser: {
                let saves_path = PathBuf::from("resources/saves")
                    .canonicalize()
                    .expect("Failed to canonicalize saves path");
                FileChooser::new(saves_path)
            },
            compiler: Compiler::new(),
            compile_error: None,
            interpreter: None,
            gif_recorder: None,
            clipboard: None,
            channel_sender,
            channel_receiver,
            reset_camera_requested: true,
            i_frame: 0,
            modifications_during_tick: 0,
            woken_up_during_tick: 0,
            #[cfg(feature = "link_ui")]
            link: "".to_string(),
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
            EditMode::DrawRect => egui::include_image!("icons/rect.png"),
            EditMode::DrawLine => egui::include_image!("icons/line.png"),
        }
    }

    pub fn view_ui(&mut self, ui: &mut egui::Ui) {
        // Edit mode choices
        ui.horizontal(|ui| {
            ui.scope(|ui| {
                const BUTTON_PADDING: f32 = 3.0;
                ui.style_mut().spacing.button_padding = egui::Vec2::splat(BUTTON_PADDING);

                for mode in EditMode::ALL {
                    let icon = Self::edit_mode_icon(mode);
                    let icon_size = egui::Vec2::splat(32.0);

                    // The behavior of Egui when clicking a button and moving the mouse is a bit weird.
                    // If a native Windows button is pressed down, the mouse moved while still inside
                    // the button and then released, it counts as a click.
                    // Egui aborts the clicked state if the mouse is moved too much. So we also consider
                    // dragging and check if the pointer is still over the button.
                    let button = egui::Button::new(icon.atom_size(icon_size))
                        .corner_radius(4)
                        .selected(mode == self.view_settings.edit_mode)
                        .sense(egui::Sense::click_and_drag());
                    let response = ui.add(button);

                    if response.clicked()
                        || response.drag_stopped() && response.hover_pos().is_some()
                    {
                        self.view_settings.edit_mode = mode;
                    }
                }
            });
        });
    }

    /// Put `material_map` into system clipboard and local clipboard (for WASM)
    pub fn clipboard_put(&mut self, ctx: &egui::Context, material_map: MaterialMap) {
        let clipboard = Clipboard::new(material_map);
        let rgba_field = material_map_effects(&clipboard.material_map, Rgba8::TRANSPARENT);

        // TODO: Currently egui does not support pasting images from the system clipboard, see
        //   https://github.com/emilk/egui/issues/2108
        //   Until the issue is resolved we copy the image as a base64 png, some image editor
        //   support pasting these, some don't.
        // Convert material_map to RgbaField and then to egui Image and put into system clipboard
        // let egui_image = egui::ColorImage::from_rgba_unmultiplied(
        //     [rgba_field.width() as usize, rgba_field.height() as usize],
        //     rgba_field.as_raw(),
        // );
        // ctx.copy_image(egui_image);

        ctx.copy_text(rgba_field.encode_base64_png());

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
            if ui.add(styled_button("🗐 Copy")).clicked() {
                self.clipboard_copy(ui.ctx());
            }

            if ui.add(styled_button("✂ Cut")).clicked() {
                self.clipboard_cut(ui.ctx());
            }

            if ui.add(styled_button("📋 Paste")).clicked() {
                if let Some(clipboard) = &self.clipboard {
                    self.view
                        .clipboard_paste(&self.view_input, clipboard.material_map.clone());
                }
                // TODO: Use ViewportCommand::RequestPaste
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub fn file_dialog_ui(&mut self, ui: &mut egui::Ui) {
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

            let bitmap = self
                .view
                .world
                .material_map()
                .to_rgba_field(Material::TRANSPARENT);

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
    pub fn file_dialog_ui(&mut self, ui: &mut egui::Ui) {
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
                self.save_to_path(path);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn file_chooser_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                let bounds = Rect::low_size(Point(0, 0), self.new_size);
                self.view = View::empty(bounds);
                ui.close();
            }
        });

        let path = self.file_chooser.show(ui);

        if ui.button("Load").clicked() {
            self.load_from_path(&path);
            ui.close();
        }

        if ui.button("Save").clicked() {
            self.save_to_path(&path);
            ui.close();
        }
    }

    pub fn gif_ui(&mut self, ui: &mut egui::Ui) {
        let n_frames = if let Some(gif_encoder) = &self.gif_recorder {
            gif_encoder.frames.len()
        } else {
            0
        };
        ui.label(format!("Frames: {n_frames}"));

        ui.horizontal(|ui| {
            if ui.button("Start").clicked() {
                self.gif_recorder = Some(GifRecorder::new());
            }

            if ui.button("Stop").clicked() {
                if let Some(gif_recorder) = self.gif_recorder.take() {
                    if let Err(err) = gif_recorder.export("movie.gif") {
                        warn!("Failed to export gif with {err}");
                    }
                }
            }

            if ui.button("Cancel").clicked() {
                self.gif_recorder = None;
            }
        });
    }

    pub fn record_gif_frame(&mut self) {
        if let Some(gif_recorder) = &mut self.gif_recorder {
            gif_recorder.add_frame(self.view.world.material_map());
        }
    }

    pub fn document_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let bounds = self.view.world.bounds();
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

    pub fn grid_size_ui(&mut self, ui: &mut egui::Ui) {
        const GRID_SIZE_CHOICES: [Option<i64>; 4] = [None, Some(4), Some(8), Some(16)];
        const GRID_SIZE_LABELS: [&str; 4] = ["None", "4px", "8px", "16px"];

        ui.label("Grid");
        ui.horizontal(|ui| {
            for (choice, label) in GRID_SIZE_CHOICES.into_iter().zip_eq(GRID_SIZE_LABELS) {
                if ui
                    .add(styled_button(label).selected(choice == self.view.grid_size))
                    .clicked()
                {
                    self.view.grid_size = choice;
                }
            }
        });
    }

    pub fn compile(&mut self) {
        info!("Compiling");
        let compiled_rules = self.compiler.compile(&self.view.world);
        match compiled_rules {
            Ok(compiled_rules) => {
                self.interpreter = Some(Interpreter::new(compiled_rules));
                self.compile_error = None;
                info!("Compiling successful");
            }
            Err(err) => {
                let bounds = err.bounds.first();
                warn!(
                    "Failed to compile with error {}, bounds: {:?}",
                    err.message, bounds
                );
                self.compile_error = Some(err);

                // todo!("Show errors as a popup over the failed rule.");
                self.interpreter = None;
            }
        }
    }

    /// Returns true if stabilized
    pub fn tick(&mut self, max_modifications: usize) {
        let Some(interpreter) = &mut self.interpreter else {
            return;
        };

        let ticked = interpreter.tick(&mut self.view.world, &self.canvas_input, max_modifications);
        let modified = match ticked {
            Ok(ticked) => ticked.changed(),
            Err(InterpreterError::MaxModificationReached) => true,
        };

        if modified {
            self.record_gif_frame();
            self.view.add_snapshot(SnapshotCause::Tick);
        }
    }

    pub fn slowmo(&mut self) {
        // Tick at lower rate than 60fps
        let tick_frame = self.i_frame % self.run_speed.frames_per_tick() == 0;
        if !tick_frame {
            return;
        }

        let Some(interpreter) = &mut self.interpreter else {
            return;
        };

        let modified = match interpreter.tick(&mut self.view.world, &self.canvas_input, 1) {
            Ok(ticked) => ticked.changed(),
            Err(_) => true,
        };

        if modified {
            self.record_gif_frame();
        }
    }

    pub fn pressed_link(&mut self) -> Option<String> {
        if !self.canvas_input.left_mouse_click {
            return None;
        }

        let topology = self.view.world.topology();
        let mouse_region = topology.region_key_at(self.canvas_input.mouse_position)?;

        let link_region =
            topology
                .iter_containing_regions(mouse_region)
                .find_map(|region_key| {
                    let region = &topology[region_key];
                    (region.material == Material::LINK).then_some(region)
                })?;

        let Ok(inner_border) = link_region.boundary.inner_borders().exactly_one() else {
            warn!("Link region has multiple hole but should only have one");
            return None;
        };

        // Copy interior of region to material map
        let encoded_address = self
            .view
            .world
            .material_map()
            .right_of_border(inner_border)
            .without(Material::BLACK);
        let bytes = encoded_address.decode_bytes();
        Some(String::from_utf8(bytes).unwrap())
    }

    pub fn run(&mut self) {
        // Tick at lower rate than 60fps
        let tick_frame = self.i_frame % self.run_speed.frames_per_tick() == 0;

        // Add gif frame if previous tick there were modifications or regions woken up
        if tick_frame && (self.modifications_during_tick > 0 || self.woken_up_during_tick > 0) {
            self.record_gif_frame();
        }

        // Check if mouse is pressed on a link
        if let Some(link) = self.pressed_link() {
            println!("{link}");
        }

        let Some(interpreter) = &mut self.interpreter else {
            return;
        };

        // Only tick once every `self.frames_per_tick`
        if self.i_frame % self.run_speed.frames_per_tick() == 0 {
            // wake up
            self.woken_up_during_tick = interpreter.wake_up(&mut self.view.world);
            self.modifications_during_tick = 0;
        }

        // Run for 10ms but don't wake up regions
        let now = Instant::now();
        while now.elapsed().as_secs_f64() < 0.01 {
            let max_modifications = 32;
            let modifications = match interpreter.stabilize(
                &mut self.view.world,
                &self.canvas_input,
                max_modifications,
            ) {
                Ok(modifications) => modifications,
                Err(InterpreterError::MaxModificationReached) => max_modifications,
            };

            if modifications == 0 {
                break;
            }

            self.modifications_during_tick += modifications;
            // let duration = now.elapsed().as_secs_f64();
            // println!(
            //     "Stabilize duration: {}, modifications: {}",
            //     duration, modifications
            // );
        }
    }

    pub fn run_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let step_button = styled_button("Step");
            if ui
                .add_enabled(self.run_mode == RunMode::Paused, step_button)
                .clicked()
            {
                // Single step
                self.compile();
                self.tick(1);
            }

            let run_button = styled_button("Run").selected(self.run_mode == RunMode::Run);
            if ui.add(run_button).clicked() {
                self.run_mode = match self.run_mode {
                    RunMode::Run => {
                        self.view.add_snapshot(SnapshotCause::Run);
                        RunMode::Paused
                    }
                    _ => {
                        self.compile();
                        RunMode::Run
                    }
                };
            }

            let walk_button = styled_button("Slowmo").selected(self.run_mode == RunMode::Slowmo);
            if ui.add(walk_button).clicked() {
                self.run_mode = match self.run_mode {
                    RunMode::Slowmo => {
                        self.view.add_snapshot(SnapshotCause::Run);
                        RunMode::Paused
                    }
                    _ => {
                        self.compile();
                        RunMode::Slowmo
                    }
                }
            }

            if self.interpreter.is_none() {
                self.run_mode = RunMode::Paused;
            }

            if self.run_mode == RunMode::Slowmo {
                self.slowmo();
            } else if self.run_mode == RunMode::Run {
                self.run();
            }
        });

        // Run speed buttons
        ui.label("Run speed");
        enum_choice_buttons(ui, None, &mut self.run_speed);

        #[cfg(not(feature = "minimal_ui"))]
        {
            // Tick button
            if ui
                .add_enabled(self.run_mode == RunMode::Paused, egui::Button::new("Tick"))
                .clicked()
            {
                self.compile();
                self.tick(1024);
            }
        }
    }

    pub fn undo_redo_ui(&mut self, ui: &mut egui::Ui) {
        let current_head = self.view.history.head.clone();
        let history = &mut self.view.history;

        // Undo, Redo buttons
        ui.horizontal(|ui| {
            if ui.add(styled_button("⟲ Undo")).clicked() {
                history.undo();
            }
            if ui.add(styled_button("⟳ Redo")).clicked() {
                history.redo();
            }
        });

        // If head has changed, update world
        if !Rc::ptr_eq(&history.head, &current_head) {
            self.view.world = World::from_material_map(history.head.material_map().clone());
            self.view.selection = history.head.selection().clone();
        }
    }

    /// If the user reverts to a snapshot in the history that snapshot is returned.
    #[cfg(any())] // disabled
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
                    let mut btn = egui::Button::new(snapshot.cause().as_str());
                    if Rc::ptr_eq(snapshot, &history.head) {
                        btn = btn.fill(Rgba8::from_rgb(Pico8Palette::ORANGE));
                    }
                    if btn.ui(ui).clicked() {
                        history.head = snapshot.clone();
                    }
                }
            });

        // If head has changed, update world
        if !Rc::ptr_eq(&history.head, &current_head) {
            self.view.world = World::from_material_map(history.head.material_map().clone());
            self.view.selection = history.head.selection().clone();
        }
    }

    pub fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        self.view_ui(ui);
        ui.separator();

        self.copy_paste_ui(ui);
        styled_space(ui);
        self.undo_redo_ui(ui);
        ui.separator();

        ui.label("Brush");
        brush_chooser(ui, &mut self.view_settings.brush);
        ui.separator();

        if let Some(prefab) = prefab_picker(ui) {
            self.view_settings.edit_mode = EditMode::SelectRect;
            self.view.paste(prefab.clone());
        }
        ui.separator();

        // ui.label("History");
        // self.history_ui(ui);
        // ui.separator();

        self.run_ui(ui);
        ui.separator();

        #[cfg(not(feature = "minimal_ui"))]
        {
            ui.label("Gif");
            self.gif_ui(ui);
            ui.separator();
        }

        #[cfg(feature = "link_ui")]
        {
            ui.text_edit_singleline(&mut self.link);
            if ui.button("Link").clicked() {
                let encoded = MaterialMap::encode_bytes(self.link.as_bytes(), Point(20, 20));
                let selection = Selection::new(encoded);
                self.view_settings.edit_mode = EditMode::SelectRect;
                self.view.set_selection(selection);
            }
        }

        self.grid_size_ui(ui);
    }

    pub fn top_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let response = ui.button("File Chooser");
                egui::Popup::menu(&response)
                    .align(egui::RectAlign::LEFT)
                    .close_behavior(egui::containers::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        self.file_chooser_ui(ui);
                    });
            }

            ui.menu_button("File", |ui| {
                self.file_dialog_ui(ui);
            });

            ui.menu_button("Document", |ui| {
                self.document_ui(ui);
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Reset camera").clicked() {
                    self.reset_camera_requested = true;
                }

                // `Camera::scale` is view_to_world, we display world_to_view scale as zoom level
                let zoom = 1.0 / self.view.camera.scale;

                let zoom_fmt = if zoom < 1.0 {
                    format!("Zoom: 1 / {}", 1.0 / zoom)
                } else {
                    format!("Zoom: {}", zoom)
                };
                ui.label(zoom_fmt);

                let world_mouse = self.view_input.world_mouse.floor();
                ui.label(format!("Mouse: {}, {}", world_mouse.x, world_mouse.y));
            });
        });
    }

    fn load_file(&mut self, content: &[u8]) {
        info!("Loading a file!");
        let Ok(rgba_field) = RgbaField::load_from_memory(content) else {
            warn!("Failed to load png file!");
            return;
        };
        self.new_size = rgba_field.size();
        let material_map = MaterialMap::from(rgba_field);
        let world = World::from(material_map);
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

    fn save_to_path(&mut self, path: impl AsRef<Path>) {
        warn!("Saving to path {:?}", path.as_ref().to_str());
        let material_map = self.view.world.material_map();
        let rgba_filed = material_map_effects(material_map, Rgba8::TRANSPARENT);
        if let Err(err) = rgba_filed.save(path) {
            warn!("Failed to save with error {err}");
        }
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

        // Show compile error
        if let Some(err) = &self.compile_error {
            // egui::Tooltip::for_widget(&response).show(|ui| {
            //     ui.label(&err.message);
            // });

            // egui::Area::new("error_popup".into()).anchor(egui::Align2::LEFT_TOP, [10.0, 10.0]).show(|ui| {
            //     ui.label(&err.message);
            // });

            let window_error_pos = if let Some(bounds) = err.bounds.first() {
                let world_error_pos = bounds.bottom_left().as_f64();
                let view_pos = self.view.camera.world_to_view() * world_error_pos;
                let window_pos = frames.view_to_window() * view_pos;
                window_pos
            } else {
                viewport.top_left()
            };

            let mut open = true;
            egui::Window::new("Error")
                .frame(
                    egui::Frame::default()
                        .fill(egui::Color32::LIGHT_RED)
                        .corner_radius(3)
                        .inner_margin(3),
                )
                .title_bar(false)
                .fixed_pos(window_error_pos)
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(&err.message);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_button = egui::Button::new("❌");
                            if ui.add(close_button).clicked() {
                                open = false;
                            }
                        });
                    });
                });

            if !open {
                self.compile_error = None;
            }
        }

        // `ctx.pointer_interact_pos()` is None if mouse is outside the window
        if let Some(egui_mouse) = ui.ctx().pointer_interact_pos() {
            let window_mouse = Point::new(egui_mouse.x as f64, egui_mouse.y as f64);
            let view_mouse = frames.window_to_view() * window_mouse;

            // Scroll captured if the mouse pointer is over view, even if it doesn't have focus.
            let scroll_delta = if response.contains_pointer() {
                input.raw_scroll_delta.y as f64
            } else {
                0.0
            };

            let wants_keyboard = ui.ctx().wants_keyboard_input();
            let hovered = response.hovered();

            let mouse = &input.pointer;

            let world_mouse = self.view.camera.view_to_world() * view_mouse;
            let left_mouse_down = hovered && mouse.button_down(egui::PointerButton::Primary);
            let right_mouse_down = hovered && mouse.button_down(egui::PointerButton::Secondary);
            let middle_mouse_down = hovered && mouse.button_down(egui::PointerButton::Middle);
            let left_mouse_click = hovered && mouse.button_clicked(egui::PointerButton::Primary);
            let right_mouse_click = hovered && mouse.button_clicked(egui::PointerButton::Secondary);

            // TODO: wants_keyboard is false when cursor is in textbox and escape is pressed, so a
            //   selection is cancelled. It should not be cancelled!
            self.view_input = ViewInput {
                frames,
                view_mouse,
                world_mouse,
                left_mouse_down,
                middle_mouse_down,
                escape_pressed: !wants_keyboard && input.key_pressed(egui::Key::Escape),
                ctrl_down: !wants_keyboard && input.modifiers.ctrl,
                delete_pressed: !wants_keyboard && input.key_pressed(egui::Key::Delete),

                mouse_wheel: scroll_delta,
            };

            self.canvas_input = CanvasInput {
                mouse_position: world_mouse.floor().as_i64(),
                left_mouse_down: left_mouse_down,
                right_mouse_down: right_mouse_down,
                left_mouse_click: left_mouse_click,
                right_mouse_click: right_mouse_click,
            }
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
            let gl = painter.gl().as_ref();

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

                view_painter.draw_view(gl, &draw_view);
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
        tracy_client::frame_mark();

        self.i_frame += 1;
        // let mut style = ctx.style().deref().clone();
        // style.visuals.dark_mode = false;
        // ctx.set_style(style);

        // Check if any files have finished loading
        #[cfg(target_arch = "wasm32")]
        if let Ok(content) = self.channel_receiver.try_recv() {
            self.load_file(&content);
        }

        ctx.style_mut(|style| {
            style.spacing.button_padding = egui::Vec2::new(8.0, 8.0);
        });

        // We cannot lock input while called clipboard_copy, clipboard_cut, ...
        // TODO: Only clone Paste, Copy, Cut events
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            match event {
                egui::Event::Paste(paste) => {
                    if let Ok(rgba_field) = RgbaField::decode_base64_png(&paste) {
                        let material_map = MaterialMap::from(rgba_field);
                        self.view.clipboard_paste(&self.view_input, material_map);
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

        self.view_settings.locked = [RunMode::Slowmo, RunMode::Run].contains(&self.run_mode);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Paused,
    Slowmo,
    Run,
}

impl RunMode {
    pub const ALL: [Self; 3] = [Self::Paused, Self::Slowmo, Self::Run];
}

impl ReflectEnum for RunMode {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Paused => "Paused",
            Self::Slowmo => "Walk",
            Self::Run => "Run",
        }
    }
}
