use crate::{
    brush::Brush,
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    field::Field,
    history::{History, SnapshotCause},
    material::Material,
    math::{
        arrow::Arrow,
        pixel::Pixel,
        point::Point,
        rect::{Rect, RectBounds},
    },
    pixmap::{MaterialMap, Pixmap},
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
    SelectRect,
}

impl EditMode {
    pub fn ui_label(self) -> &'static str {
        match self {
            Self::Brush => "Brush",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill",
            Self::SelectRect => "Select Rect",
        }
    }

    pub const ALL: [EditMode; 4] = [Self::Brush, Self::Eraser, Self::Fill, Self::SelectRect];
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
pub struct SelectingRect {
    pub start: Point<f64>,
    pub stop: Point<f64>,
}

#[derive(Debug, Clone)]
pub struct MovingSelection {
    pub previous: Point<i64>,
}

impl SelectingRect {
    pub fn rect(&self) -> Rect<i64> {
        // end can be smaller than start, so we take the bounds of both points
        [self.start.cwise_cast(), self.stop.cwise_cast()].bounds()
    }
}

#[derive(Debug, Clone)]
pub enum UiState {
    MoveCamera(MoveCamera),
    Brushing(Brushing),
    SelectingRect(SelectingRect),
    MovingSelection(MovingSelection),
    Idle,
}

impl UiState {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

#[derive(Debug, Clone)]
pub struct Selection {
    /// The bounding rect of material_map
    bounding_rect: Rect<i64>,

    material_map: MaterialMap,

    offset: Point<i64>,
}

impl Selection {
    pub fn new(material_map: MaterialMap) -> Self {
        let rect = material_map.bounding_rect();
        Self {
            material_map,
            bounding_rect: rect,
            offset: Point(0, 0),
        }
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.bounding_rect + self.offset
    }

    pub fn with_center_at(self, center: Point<i64>) -> Self {
        let current_center = self.bounding_rect().center();
        self.translated(center - current_center)
    }

    pub fn translated(mut self, offset: Point<i64>) -> Self {
        self.offset = self.offset + offset;
        self
    }

    pub fn blit(&self, target: &mut MaterialMap) {
        // TODO:SPEEDUP: Should be done in MaterialMap
        for (index, &material) in self.material_map.iter() {
            target.try_set(index + self.offset, material).ok();
        }
    }
}

pub struct View {
    pub world: World,
    pub history: History<Material>,
    pub camera: Camera,

    pub selection: Option<Selection>,

    pub ui_state: UiState,
}

impl View {
    pub fn new(world: World) -> View {
        let history = History::new(world.material_map().clone());
        View {
            world,
            history,
            camera: Camera::default(),
            ui_state: UiState::Idle,
            selection: None,
        }
    }

    pub fn center_camera(&mut self, view_rect: Rect<f64>) {
        let mut world_bounds = self.world.bounding_rect();
        if world_bounds.is_empty() {
            world_bounds = Rect::low_high([-128, -128], [128, 128])
        }

        self.camera = Camera::fit_world_into_view(world_bounds.cwise_cast(), view_rect).round();
    }

    pub fn empty() -> View {
        let field = Field::filled(Rect::low_size([0, 0], [512, 512]), Material::TRANSPARENT);
        let material_map = Pixmap::from_field(&field);
        let world = World::from_material_map(material_map);
        Self::new(world)
    }

    /// If there currently is a selection, cancel it and integrate its content back into the world
    pub fn cancel_selection(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.world
                .mut_material_map(|material_map| selection.blit(material_map));
        }
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
            let cause = if op.brush.material == Material::TRANSPARENT {
                SnapshotCause::Erase
            } else {
                SnapshotCause::Brush
            };
            self.history
                .add_snapshot(self.world.material_map().clone(), cause);
            return UiState::Idle;
        }

        if !input.left_mouse.is_down {
            return UiState::Idle;
        }

        let change = op.brush.draw_line(Arrow(op.world_mouse, input.world_mouse));
        op.world_mouse = input.world_mouse;

        self.world.mut_material_map(|material_map| {
            for (&index, &material) in &change {
                material_map.try_set(index, material).ok();
            }
        });

        UiState::Brushing(op)
    }

    pub fn handle_selecting_rect(
        &mut self,
        mut op: SelectingRect,
        input: &ViewInput,
        _settings: &ViewSettings,
    ) -> UiState {
        if input.left_mouse.is_up() {
            self.cancel_selection();

            // set selection by cutting the selected rectangle out of the world
            let selection_rect = op.rect();
            let selection = self.world.material_map().sub_rect(selection_rect);
            self.world.fill_rect(selection_rect, Material::TRANSPARENT);
            self.selection = Some(Selection::new(selection));
            return UiState::Idle;
        }

        op.stop = input.world_mouse;
        UiState::SelectingRect(op)
    }

    pub fn handle_moving_selection(
        &mut self,
        mut op: MovingSelection,
        input: &ViewInput,
        _settings: &ViewSettings,
    ) -> UiState {
        if input.left_mouse.is_up() {
            return UiState::Idle;
        }

        // Move selection by mouse travelled
        let current = input.world_mouse.floor().cwise_cast::<i64>();
        let delta = current - op.previous;
        self.selection = self
            .selection
            .take()
            .map(|selection| selection.translated(delta));
        op.previous = current;
        UiState::MovingSelection(op)
    }

    pub fn begin_selection_action(
        &mut self,
        input: &ViewInput,
        settings: &ViewSettings,
    ) -> UiState {
        // If mouse is inside the current selecting we enter the MoveSelection move.
        if let Some(selection) = &self.selection {
            if selection
                .bounding_rect()
                .cwise_cast::<f64>()
                .contains(input.world_mouse)
            {
                let op = MovingSelection {
                    previous: input.world_mouse.floor().cwise_cast(),
                };
                return UiState::MovingSelection(op);
            }
        }

        // Otherwise we start drawing a new selection rectangle.
        let op = SelectingRect {
            start: input.world_mouse,
            stop: input.world_mouse,
        };
        return self.handle_selecting_rect(op, input, settings);
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
                        brush.material = Material::TRANSPARENT;
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
                    let pixel: Pixel = input.world_mouse.floor().cwise_cast().into();
                    if let Some(region_key) = self.world.topology().region_at(pixel) {
                        self.world.fill_region(region_key, settings.brush.material);
                        self.history
                            .add_snapshot(self.world.material_map().clone(), SnapshotCause::Fill);
                    }
                }
            }
            EditMode::SelectRect => {
                if input.left_mouse.is_down {
                    return self.begin_selection_action(input, settings);
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

        if input.escape_key.is_down {
            self.cancel_selection();
        }

        // if input.delete_key.is_down {
        //     self.delete_selection();
        // }

        let ui_state = self.ui_state.clone();
        self.ui_state = match ui_state {
            UiState::MoveCamera(op) => self.handle_moving_camera(op, input, settings),
            UiState::Brushing(op) => self.handle_brushing(op, input, settings),
            UiState::SelectingRect(op) => self.handle_selecting_rect(op, input, settings),
            UiState::MovingSelection(op) => self.handle_moving_selection(op, input, settings),
            UiState::Idle => self.begin_action(input, settings),
        };
    }

    pub fn tile_containing(&self, world_point: Point<f64>) -> Point<i64> {
        world_point.floor().cwise_cast()
    }

    pub fn is_hovering_selection(&self, view_input: &ViewInput) -> bool {
        if let Some(selection) = &self.selection {
            selection
                .bounding_rect()
                .cwise_cast::<f64>()
                .contains(view_input.world_mouse)
        } else {
            false
        }
    }

    pub fn clipboard_copy(&mut self) -> Option<MaterialMap> {
        let Some(selection) = &self.selection else {
            return None;
        };

        Some(selection.material_map.clone())
    }

    pub fn clipboard_cut(&mut self) -> Option<MaterialMap> {
        let Some(selection) = self.selection.take() else {
            return None;
        };

        Some(selection.material_map)
    }

    pub fn clipboard_paste(&mut self, input: &ViewInput, material_map: MaterialMap) {
        self.cancel_selection();
        // Create selection from entry
        let world_mouse = input.world_mouse.floor().cwise_cast();
        let selection = Selection::new(material_map).with_center_at(world_mouse);
        self.selection = Some(selection);
    }
}
