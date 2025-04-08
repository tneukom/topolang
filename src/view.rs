use crate::{
    brush::Brush,
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    field::{Field, RgbaField},
    history::{History, SnapshotCause},
    material::Material,
    material_effects::material_map_effects,
    math::{arrow::Arrow, pixel::Pixel, point::Point, rect::Rect, rgba8::Rgba8},
    pixmap::{MaterialMap, Pixmap},
    world::World,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ViewInput {
    pub frames: CoordinateFrames,

    pub view_mouse: Point<f64>,

    pub world_mouse: Point<f64>,
    pub world_snapped: Point<f64>,

    pub left_mouse_down: bool,
    pub middle_mouse_down: bool,

    pub escape_pressed: bool,
    pub ctrl_down: bool,
    pub delete_pressed: bool,

    pub mouse_wheel: f64,
}

impl ViewInput {
    pub const EMPTY: Self = Self {
        frames: CoordinateFrames {
            window_size: Point(640.0, 480.0),
            viewport: Rect::low_high(Point::ZERO, Point(640.0, 480.0)),
        },

        view_mouse: Point::ZERO,

        world_mouse: Point::ZERO,
        world_snapped: Point::ZERO,

        left_mouse_down: false,
        middle_mouse_down: false,

        escape_pressed: false,
        ctrl_down: false,
        delete_pressed: false,

        mouse_wheel: 0.0,
    };

    /// Either middle mouse or ctrl + left mouse
    pub fn move_camera_down(&self) -> bool {
        self.middle_mouse_down || (self.left_mouse_down && self.ctrl_down)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditMode {
    Brush,
    Eraser,
    Fill,
    SelectRect,
    SelectWand,
    PickColor,
}

impl EditMode {
    pub fn ui_label(self) -> &'static str {
        match self {
            Self::Brush => "Brush",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill",
            Self::SelectRect => "Select Rect",
            Self::SelectWand => "Select Wand",
            Self::PickColor => "Pick Color",
        }
    }

    pub const ALL: [EditMode; 6] = [
        Self::Brush,
        Self::Eraser,
        Self::Fill,
        Self::SelectRect,
        Self::SelectWand,
        Self::PickColor,
    ];

    pub fn is_select(self) -> bool {
        self == Self::SelectWand || self == Self::SelectRect
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectingKind {
    Rect,
    Wand,
}

#[derive(Debug, Clone)]
pub struct Selecting {
    pub kind: SelectingKind,

    pub world_start: Point<f64>,
    pub world_stop: Point<f64>,

    pub view_start: Point<f64>,
    pub view_stop: Point<f64>,
}

#[derive(Debug, Clone)]
pub struct MovingSelection {
    pub previous: Point<i64>,
}

impl Selecting {
    pub fn rect(&self) -> Rect<i64> {
        // end can be smaller than start, so we take the bounds of both points
        Rect::index_bounds([self.world_start.as_i64(), self.world_stop.as_i64()])
    }
}

#[derive(Debug, Clone)]
pub enum UiState {
    MoveCamera(MoveCamera),
    Brushing(Brushing),
    SelectingRect(Selecting),
    MovingSelection(MovingSelection),
    Idle,
}

impl UiState {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    material_map: MaterialMap,

    /// Arc because we share it with the renderer.
    rgba_field: Arc<RgbaField>,
}

impl Selection {
    pub fn new(material_map: MaterialMap) -> Self {
        let mut rgba_field = RgbaField::filled(material_map.bounding_rect(), Rgba8::TRANSPARENT);
        material_map_effects(&material_map, &mut rgba_field);
        Self {
            material_map,
            rgba_field: Arc::new(rgba_field),
        }
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.material_map.bounding_rect()
    }

    pub fn is_empty(&self) -> bool {
        !self.material_map.bounding_rect().has_positive_area()
    }

    pub fn with_center_at(self, center: Point<i64>) -> Self {
        let current_center = self.bounding_rect().center();
        self.translated(center - current_center)
    }

    pub fn translated(mut self, offset: Point<i64>) -> Self {
        self.material_map = self.material_map.translated(offset);
        // Clones rgba_field when renderer currently uses it, should not be an issue.
        let rgba_field = Arc::unwrap_or_clone(self.rgba_field);
        self.rgba_field = Arc::new(rgba_field.translated(offset));
        self
    }

    pub fn material_map(&self) -> &MaterialMap {
        &self.material_map
    }

    pub fn rgba_field(&self) -> &Arc<RgbaField> {
        &self.rgba_field
    }

    // pub fn blit_to(&self, target: &mut MaterialMap) {
    //     // Ignore transparent
    //     target.blit(&self.material_map.without(Material::TRANSPARENT));
    // }
}

pub struct View {
    pub world: World,
    pub history: History,
    pub camera: Camera,
    pub grid_size: Option<i64>,

    pub selection: Option<Selection>,

    pub ui_state: UiState,
}

impl View {
    pub fn new(world: World) -> View {
        let history = History::new(world.material_map().clone(), None);
        View {
            world,
            history,
            camera: Camera::default(),
            grid_size: None,
            ui_state: UiState::Idle,
            selection: None,
        }
    }

    pub fn add_snapshot(&mut self, cause: SnapshotCause) {
        self.history.add_snapshot(
            self.world.material_map().clone(),
            self.selection.clone(),
            cause,
        );
    }

    pub fn center_camera(&mut self, view_rect: Rect<f64>) {
        let mut world_bounds = self.world.bounding_rect();
        if world_bounds.is_empty() {
            world_bounds = Rect::low_high(Point(-128, -128), Point(128, 128))
        }

        self.camera = Camera::fit_world_into_view(world_bounds.cwise_as(), view_rect).round();
    }

    pub fn empty(bounds: Rect<i64>) -> View {
        let field = Field::filled(bounds, Material::TRANSPARENT);
        let material_map = Pixmap::from_field(&field);
        let world = World::from_material_map(material_map);
        Self::new(world)
    }

    /// If there currently is a selection, cancel it and integrate its content back into the world
    pub fn cancel_selection(&mut self) {
        if let Some(selection) = self.selection.take() {
            let selection_material_map = selection.material_map().without(Material::TRANSPARENT);
            self.world.blit(&selection_material_map);
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
        let stop = mode_exited || !input.left_mouse_down;
        if stop {
            let cause = if op.brush.material == Material::TRANSPARENT {
                SnapshotCause::Erase
            } else {
                SnapshotCause::Brush
            };
            self.add_snapshot(cause);

            return UiState::Idle;
        }

        if !input.left_mouse_down {
            return UiState::Idle;
        }

        let change = op.brush.draw_line(Arrow(op.world_mouse, input.world_mouse));
        op.world_mouse = input.world_mouse;

        let change_pixmap = Pixmap::from(&change);
        self.world.blit(&change_pixmap);

        UiState::Brushing(op)
    }

    pub fn handle_selecting(
        &mut self,
        mut op: Selecting,
        input: &ViewInput,
        _settings: &ViewSettings,
    ) -> UiState {
        if !input.left_mouse_down {
            self.cancel_selection();

            let selection = match op.kind {
                SelectingKind::Rect => {
                    if op.view_start.distance(op.view_stop) < 5.0 {
                        self.add_snapshot(SnapshotCause::SelectionCancelled);
                        return UiState::Idle;
                    }

                    self.world.rect_selection(op.rect())
                }
                SelectingKind::Wand => {
                    let pixel: Pixel = input.world_mouse.floor().as_i64();
                    let Some(region_key) = self.world.topology().region_key_at(pixel) else {
                        return UiState::Idle;
                    };

                    if self.world.topology().regions[&region_key].material == Material::TRANSPARENT
                    {
                        // Don't let the user select transparent regions
                        return UiState::Idle;
                    }

                    self.world.region_selection(region_key)
                }
            };

            self.selection = Some(selection);
            self.add_snapshot(SnapshotCause::Selected);

            return UiState::Idle;
        }

        op.world_stop = input.world_mouse;
        op.view_stop = input.view_mouse;
        UiState::SelectingRect(op)
    }

    pub fn handle_moving_selection(
        &mut self,
        mut op: MovingSelection,
        input: &ViewInput,
        _settings: &ViewSettings,
    ) -> UiState {
        if !input.left_mouse_down {
            self.add_snapshot(SnapshotCause::SelectionMoved);
            return UiState::Idle;
        }

        // Move selection by mouse travelled
        let current = input.world_mouse.floor().cwise_as::<i64>();
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
        kind: SelectingKind,
    ) -> UiState {
        // If mouse is inside the current selecting we enter the MoveSelection move.
        if let Some(selection) = &self.selection {
            if selection
                .bounding_rect()
                .cwise_as::<f64>()
                .contains(input.world_mouse)
            {
                let op = MovingSelection {
                    previous: input.world_mouse.floor().cwise_as(),
                };
                return UiState::MovingSelection(op);
            }
        }

        // Otherwise we start drawing a new selection rectangle.
        let op = Selecting {
            kind,
            world_start: input.world_mouse,
            world_stop: input.world_mouse,
            view_start: input.view_mouse,
            view_stop: input.view_mouse,
        };
        self.handle_selecting(op, input, settings)
    }

    /// Transition from None state
    pub fn begin_action(&mut self, input: &ViewInput, settings: &mut ViewSettings) -> UiState {
        if input.move_camera_down() {
            let move_camera = MoveCamera {
                world_mouse: input.world_mouse,
            };
            return UiState::MoveCamera(move_camera);
        }

        match settings.edit_mode {
            EditMode::Brush | EditMode::Eraser => {
                if input.left_mouse_down {
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
                if input.left_mouse_down {
                    let pixel: Pixel = input.world_mouse.floor().as_i64();
                    if let Some(region_key) = self.world.topology().region_key_at(pixel) {
                        self.world.fill_region(region_key, settings.brush.material);
                        self.add_snapshot(SnapshotCause::Fill);
                    }
                }
            }
            EditMode::SelectRect => {
                if input.left_mouse_down {
                    return self.begin_selection_action(input, settings, SelectingKind::Rect);
                }
            }
            EditMode::SelectWand => {
                if input.left_mouse_down {
                    return self.begin_selection_action(input, settings, SelectingKind::Wand);
                }
            }
            EditMode::PickColor => {
                if input.left_mouse_down {
                    let pixel: Pixel = input.world_mouse.floor().as_i64();
                    if let Some(material) = self.world.material_map().get(pixel) {
                        settings.brush.material = material;
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
        if input.mouse_wheel < 0.0 {
            self.camera = self.camera.zoom_in_at_view_point(input.view_mouse).round();
        } else if input.mouse_wheel > 0.0 {
            self.camera = self.camera.zoom_out_at_view_point(input.view_mouse).round();
        }
        input.world_mouse = self.camera.view_to_world() * input.view_mouse;
        input.world_snapped = self.nearest_grid_vertex(input.world_mouse);
    }

    pub fn handle_input(&mut self, input: &mut ViewInput, settings: &mut ViewSettings) {
        self.handle_camera_input(input);

        if input.escape_pressed {
            self.cancel_selection();
            self.add_snapshot(SnapshotCause::SelectionCancelled);
        }

        if !settings.edit_mode.is_select() {
            self.cancel_selection();
        }

        if input.delete_pressed {
            self.selection = None;
        }

        let ui_state = self.ui_state.clone();
        self.ui_state = match ui_state {
            UiState::MoveCamera(op) => self.handle_moving_camera(op, input, settings),
            UiState::Brushing(op) => self.handle_brushing(op, input, settings),
            UiState::SelectingRect(op) => self.handle_selecting(op, input, settings),
            UiState::MovingSelection(op) => self.handle_moving_selection(op, input, settings),
            UiState::Idle => self.begin_action(input, settings),
        };
    }

    pub fn tile_containing(&self, world_point: Point<f64>) -> Point<i64> {
        world_point.floor().cwise_as()
    }

    pub fn is_hovering_selection(&self, view_input: &ViewInput) -> bool {
        if let Some(selection) = &self.selection {
            selection
                .bounding_rect()
                .cwise_as::<f64>()
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
        let world_mouse = input.world_mouse.floor().cwise_as();
        let selection = Selection::new(material_map).with_center_at(world_mouse);
        self.selection = Some(selection);
    }

    pub fn resize(&mut self, bounds: Rect<i64>) {
        let mut resized = MaterialMap::filled(bounds, Material::TRANSPARENT);
        resized.blit(self.world.material_map());
        self.world = World::from_material_map(resized);

        self.add_snapshot(SnapshotCause::Resized);
    }
}
