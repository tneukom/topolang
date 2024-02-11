use egui::{epaint, load::SizedTexture, Sense, TextureOptions, Widget};
use std::{collections::HashMap, hash::Hash, path::PathBuf, sync::Arc};

use crate::{
    bitmap::Bitmap,
    brush::Brush,
    coordinate_frame::CoordinateFrames,
    interpreter::Interpreter,
    math::{point::Point, rect::Rect},
    painting::scene_painter::ScenePainter,
    topology::Topology,
    view::{EditMode, View, ViewButton, ViewInput, ViewSettings},
    widgets::{system_colors_widget, ColorChooser, FileChooser},
};
use glow::HasContext;
use instant::Instant;

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

pub struct EguiApp {
    scene_painter: ScenePainter,
    start_time: Instant,
    pub view_settings: ViewSettings,

    view_rect: Rect<i64>,

    gl: Arc<glow::Context>,
    view: View,
    mouse_button_states: HashMap<MouseButton, MouseButtonState>,

    view_input: ViewInput,

    file_name: String,
    // current_folder: PathBuf,
    run: bool,
    interpreter: Interpreter,

    // stabilize: bool,
    // stabilize_count: i64,
    edit_mode_icons: HashMap<EditMode, egui::TextureHandle>,

    file_chooser: FileChooser,
    color_chooser: ColorChooser,
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
        let scene_painter = ScenePainter::new(cc.gl.as_ref().unwrap().clone());
        let start_time = Instant::now();

        // Load topology from file
        // TODO: Should be fetched instead of included
        let world_image_bytes = include_bytes!("../resources/saves/circles.png");
        let world_bitmap = Bitmap::load_from_memory(world_image_bytes).unwrap();
        // let world_bitmap = Bitmap::transparent(512, 512);
        let world = Topology::from_bitmap(&world_bitmap);
        // let world = Topology::from_bitmap_path("test_resources/compiler/gate/world.png").unwrap();
        let view = View::new(world);

        let mouse_button_states: HashMap<_, _> = MouseButton::VALUES
            .into_iter()
            .map(|button| (button, MouseButtonState::Up))
            .collect();

        let gl = cc.gl.clone().unwrap();

        let load_egui_icon = |name: &str, bytes: &[u8]| {
            let bitmap = Bitmap::load_from_memory(bytes).unwrap();
            let raw: &[u8] = &bitmap.as_raw();
            let size = [bitmap.width(), bitmap.height()];
            let egui_image = egui::ColorImage::from_rgba_unmultiplied(size, raw);

            cc.egui_ctx
                .load_texture(name, egui_image, TextureOptions::LINEAR)
        };

        let edit_mode_icons = HashMap::from([(
            EditMode::Brush,
            load_egui_icon("icons/brush.png", include_bytes!("icons/brush.png")),
        )]);

        let view_settings = ViewSettings {
            edit_mode: EditMode::Brush,
            brush: Brush::default(),
        };

        Self {
            scene_painter,
            start_time,
            view,
            view_settings,
            view_rect: Rect::low_size([0, 0], [1, 1]),
            mouse_button_states,
            gl,
            file_name: "".to_string(),
            run: false,
            // stabilize: false,
            // stabilize_count: 0,
            // current_folder: PathBuf::from("resources/saves"),
            view_input: ViewInput::EMPTY,
            edit_mode_icons,
            file_chooser: FileChooser::new(PathBuf::from("resources/saves")),
            color_chooser: ColorChooser::default(),
            interpreter: Interpreter::new(),
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

        let pixelwindow_mouse = match ctx.pointer_latest_pos() {
            Some(egui_mouse) => Point::new(egui_mouse.x as f64, egui_mouse.y as f64),
            None => Point::ZERO,
        };

        let frames = Self::frames(ctx);
        let view_mouse = frames.pixelwindow_to_view() * pixelwindow_mouse;

        let scroll_delta = if ctx.wants_pointer_input() {
            0.0
        } else {
            ctx.input(|input| input.scroll_delta.y as f64 / 50.0)
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

    pub fn view_ui(&mut self, ui: &mut egui::Ui) {
        // Mouse world position
        ui.label(format!(
            "snapped mouse: {:?}",
            self.view_input.world_snapped
        ));

        if ui.button("Reset camera").clicked() {
            self.view.center_camera(self.view_rect.cwise_into_lossy());
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

                if response.clicked() || response.drag_released() && response.hover_pos().is_some()
                {
                    self.view_settings.edit_mode = mode;
                }
            }
        });
    }

    pub fn load_save_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("New").clicked() {
            self.view = View::empty();
        }

        let path = self.file_chooser.show(ui);

        if ui.button("Load").clicked() {
            //let path = format!("resources/saves/{}", self.file_name);
            println!("Loading file {:?} in folder", &path);

            todo!("Load from bitmap file");
            ui.close_menu();
        }

        if ui.button("Save").clicked() {
            println!("Saving edit scene to {}", self.file_name);

            todo!("Implement save");
            ui.close_menu();
        }
    }

    pub fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        self.view_ui(ui);
        ui.separator();

        ui.label("Brush");

        // Brush color
        self.color_chooser.show(ui);
        self.view_settings.brush.color = self.color_chooser.color;

        ui.label("System colors");
        system_colors_widget(ui, &mut self.color_chooser.color);

        // Brush shape
        ui.add(egui::Slider::new(&mut self.view_settings.brush.radius, 0..=5).text("Radius"));
        ui.separator();

        // Step and run
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.run, egui::Button::new("Step"))
                .clicked()
            {
                self.interpreter.step(&mut self.view.world);
            }

            if egui::Button::new("Run").selected(self.run).ui(ui).clicked() {
                self.run = !self.run;
            }
        });

        if self.run {
            self.interpreter.step(&mut self.view.world);
        }
    }

    pub fn top_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                self.load_save_ui(ui);
            });
        });
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // let mut style = ctx.style().deref().clone();
        // style.visuals.dark_mode = false;
        // ctx.set_style(style);

        let mut visual = egui::Visuals::light();
        visual.window_shadow = epaint::Shadow::small_light();
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
        // println!("side_panel_rect left: {}, right: {}", side_panel_rect.left(), side_panel_rect.right());
        self.view_rect = Rect::low_high([side_panel_rect.right() as i64, 0], view_size);

        // view_input after the UI has been drawn, so we know if the cursor is
        // over an element.
        self.view_input = self.view_input_from(ctx);
        let time = self.time();

        self.view
            .handle_input(&mut self.view_input, &self.view_settings);
        let frames = self.view_input.frames();
        // let preview = self.view.preview(&self.view_settings);

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

            self.scene_painter.draw_grid(&self.view.camera, &frames);

            let pixmap = self.view.world.to_pixmap_without_transparent();
            self.scene_painter
                .draw_pixmap(&pixmap, &self.view.camera, &frames);

            let bounds = self.view.world.bounds();
            self.scene_painter
                .draw_bounds(bounds, &self.view.camera, &frames, time);
        }

        self.scene_painter.i_frame += 1;

        ctx.request_repaint();
    }
}
