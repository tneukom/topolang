use crate::{
    brush::Brush,
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    field::Field,
    history::{History, SnapshotCause},
    math::{arrow::Arrow, pixel::Pixel, point::Point, rect::Rect, rgba8::Rgba8},
    pixmap::Pixmap,
    world::World,
};

#[derive(Debug, Clone, Copy)]
pub struct ViewButton {
    pub is_down: bool,
    pub is_pressed: bool,
}

impl ViewButton {
    pub const fn up() -> Self {
        Self {
            is_down: false,
            is_pressed: false,
        }
    }

    pub fn is_up(&self) -> bool {
        !self.is_down
    }
}

#[derive(Debug, Clone)]
pub struct ViewInput {
    pub view_size: Point<i64>,
    pub view_mouse: Point<f64>,

    pub world_mouse: Point<f64>,
    pub world_snapped: Point<f64>,

    pub left_mouse: ViewButton,
    pub middle_mouse: ViewButton,
    pub right_mouse: ViewButton,

    pub shift_key: ViewButton,
    pub ctrl_key: ViewButton,
    pub delete_key: ViewButton,
    pub enter_key: ViewButton,
    pub escape_key: ViewButton,

    pub mouse_wheel: f64,
}

impl ViewInput {
    pub fn frames(&self) -> CoordinateFrames {
        CoordinateFrames::new(self.view_size.x, self.view_size.y)
    }

    pub const EMPTY: Self = Self {
        view_size: Point::new(100, 100),
        view_mouse: Point::ZERO,

        world_mouse: Point::ZERO,
        world_snapped: Point::ZERO,

        left_mouse: ViewButton::up(),
        middle_mouse: ViewButton::up(),
        right_mouse: ViewButton::up(),

        shift_key: ViewButton::up(),
        ctrl_key: ViewButton::up(),
        delete_key: ViewButton::up(),
        enter_key: ViewButton::up(),
        escape_key: ViewButton::up(),

        mouse_wheel: 0.0,
    };

    /// Either middle mouse or ctrl + left mouse
    pub fn move_camera_down(&self) -> bool {
        self.middle_mouse.is_down || (self.left_mouse.is_down && self.ctrl_key.is_down)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditMode {
    Brush,
    Eraser,
    Fill,
}

impl EditMode {
    pub fn ui_label(self) -> &'static str {
        match self {
            Self::Brush => "Brush",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill",
        }
    }

    pub const ALL: [EditMode; 3] = [Self::Brush, Self::Eraser, Self::Fill];
}

#[derive(Debug, Clone)]
pub struct ViewSettings {
    pub edit_mode: EditMode,
    pub brush: Brush,
}

#[derive(Debug, Clone)]
pub struct MoveCamera {
    world_mouse: Point<f64>,
}

#[derive(Debug, Clone)]
pub struct Brushing {
    pub brush: Brush,
    pub world_mouse: Point<f64>,
}

#[derive(Debug, Clone)]
pub enum UiState {
    MoveCamera(MoveCamera),
    Brushing(Brushing),
    Idle,
}

pub struct View {
    pub world: World,
    pub history: History,
    pub camera: Camera,

    pub ui_state: UiState,
}

impl View {
    pub fn new(world: World) -> View {
        let history = History::new(world.color_map().clone());
        View {
            world,
            history,
            camera: Camera::default(),
            ui_state: UiState::Idle,
        }
    }

    pub fn center_camera(&mut self, view_rect: Rect<f64>) {
        let mut world_bounds = self.world.bounding_rect();
        if world_bounds.is_empty() {
            world_bounds = Rect::low_high([-128, -128], [128, 128])
        }

        self.camera =
            Camera::fit_world_into_view(world_bounds.cwise_into_lossy(), view_rect).round();
    }

    pub fn empty() -> View {
        let field = Field::filled(Rect::low_size([0, 0], [512, 512]), Rgba8::TRANSPARENT);
        let color_map = Pixmap::from_field(&field);
        let world = World::from_pixmap(color_map);
        Self::new(world)
    }

    pub fn handle_moving_camera(
        &mut self,
        op: MoveCamera,
        input: &ViewInput,
        _settings: &ViewSettings,
    ) -> UiState {
        if !input.move_camera_down() {
            return UiState::Idle;
        }

        // The world point where the mouse was when the MoveCamera started should be the
        // world point where the mouse is now.
        self.camera =
            Camera::map_view_to_world(input.view_mouse, op.world_mouse, self.camera.scale.clone())
                .round();
        UiState::MoveCamera(op)
    }

    fn handle_brushing(
        &mut self,
        mut op: Brushing,
        input: &ViewInput,
        settings: &ViewSettings,
    ) -> UiState {
        // Because the brushing op is started even if left mouse is not down, we need to exit if
        // mode changes.
        let mode_exited = ![EditMode::Brush, EditMode::Eraser].contains(&settings.edit_mode);
        let stop = mode_exited || !input.left_mouse.is_down;
        if stop {
            let cause = if op.brush.color == Rgba8::TRANSPARENT {
                SnapshotCause::Erase
            } else {
                SnapshotCause::Brush
            };
            self.history
                .add_snapshot(self.world.color_map().clone(), cause);
            return UiState::Idle;
        }

        if !input.left_mouse.is_down {
            return UiState::Idle;
        }

        let change = op.brush.draw_line(Arrow(op.world_mouse, input.world_mouse));
        op.world_mouse = input.world_mouse;

        self.world.blit_over(&change);

        UiState::Brushing(op)
    }

    /// Transition from None state
    pub fn begin_action(&mut self, input: &ViewInput, settings: &ViewSettings) -> UiState {
        if input.move_camera_down() {
            let move_camera = MoveCamera {
                world_mouse: input.world_mouse,
            };
            return UiState::MoveCamera(move_camera);
        }

        match settings.edit_mode {
            EditMode::Brush | EditMode::Eraser => {
                if input.left_mouse.is_down {
                    let mut brush = settings.brush;
                    // Clear is brushing with transparent color
                    if settings.edit_mode == EditMode::Eraser {
                        brush.color = Rgba8::TRANSPARENT;
                    }

                    let op = Brushing {
                        world_mouse: input.world_mouse,
                        brush,
                    };
                    return self.handle_brushing(op, input, settings);
                }
            }
            EditMode::Fill => {
                if input.left_mouse.is_down {
                    let pixel: Pixel = input.world_mouse.floor().cwise_into_lossy().into();
                    if let Some(region_key) = self.world.topology().region_at(pixel) {
                        self.world.fill_region(region_key, settings.brush.color);
                        self.history
                            .add_snapshot(self.world.color_map().clone(), SnapshotCause::Fill);
                    }
                }
            }
        }

        UiState::Idle
    }

    pub fn nearest_grid_vertex(&self, world_point: Point<f64>) -> Point<f64> {
        world_point.round()
    }

    fn handle_camera_input(&mut self, input: &mut ViewInput) {
        let scale_delta = 2.0_f64.powf(-input.mouse_wheel);
        self.camera = self
            .camera
            .zoom_at_view_point(input.view_mouse, scale_delta)
            .round();
        input.world_mouse = self.camera.view_to_world() * input.view_mouse;
        input.world_snapped = self.nearest_grid_vertex(input.world_mouse);
    }

    pub fn handle_input(&mut self, input: &mut ViewInput, settings: &ViewSettings) {
        self.handle_camera_input(input);

        // if input.delete_key.is_down {
        //     self.delete_selection();
        // }

        let ui_state = self.ui_state.clone();
        self.ui_state = match ui_state {
            UiState::MoveCamera(op) => self.handle_moving_camera(op, input, settings),
            UiState::Brushing(op) => self.handle_brushing(op, input, settings),
            UiState::Idle => self.begin_action(input, settings),
        };
    }

    pub fn tile_containing(&self, world_point: Point<f64>) -> Point<i64> {
        world_point.floor().cwise_into_lossy()
    }

    // /// Preview of the current drawing operation
    // pub fn preview(&self, settings: &ViewSettings) -> TileMap {
    //     let mut tile_map = TileMap::empty();
    //     match &self.ui_state {
    //         UiState::Brushing(op) => op.draw(&mut tile_map, settings),
    //         _ => {}
    //     }
    //
    //     tile_map
    // }
}
