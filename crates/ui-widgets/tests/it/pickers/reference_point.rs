//! `ReferencePoint` marks its nine points with the dot the picker has always shown; asked for a presentation, it marks
//! them with the squares of the box's grid, joined to their neighbours, and keeps every part, cell, name and pick.

use craft_behavior::Anchor9;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::pickers::{PointPresentation, ReferencePoint};
use craft_ui::testing::Fixture;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::trial;
use egui::epaint::RectShape;
use egui::{Key, Modifiers, Pos2, Rect, Shape, Ui};
use std::cell::Cell;
use std::rc::Rc;

/// Three points on each side of the picker's grid.
const GRID: usize = 3;

/// The centre point's place among the nine, which the fixtures pick.
const CENTRE: usize = 4;

/// The slack a painted shape is compared with: the rounding of the layout.
const ROUNDING: f32 = 0.5;

/// The nine points' names, in the order the picker declares them: row by row from the top left.
const NAMES: [&str; GRID * GRID] = ["Top left", "Top", "Top right", "Left", "Center", "Right", "Bottom left", "Bottom", "Bottom right"];

/// The picker a trial shows, holding the point it edits and the mark it shows it with.
struct Picker {
    point: Cell<Anchor9>,
    presentation: PointPresentation,
    compact: bool,
    growth: Option<(f64, f64)>,
}

impl Picker {
    /// The picker as the library shows it, the centre picked.
    fn dots() -> Self {
        Picker { point: Cell::new(Anchor9::Centre), presentation: PointPresentation::Dots, compact: false, growth: None }
    }

    fn squares() -> Self {
        Picker { presentation: PointPresentation::ConnectedSquares, ..Picker::dots() }
    }

    /// The picker the canvas grows both ways about, so every point beside the centre has an arrow.
    fn grown(self) -> Self {
        Picker { growth: Some((1.0, 1.0)), ..self }
    }

    fn compact(self) -> Self {
        Picker { compact: true, ..self }
    }

    fn show(&self, ui: &mut Ui) {
        let mut point = self.point.get();
        let mut picker = ReferencePoint::new("Reference point", Bind::bound(&mut point)).presentation(self.presentation);
        if self.compact {
            picker = picker.compact();
        }
        if let Some(growth) = self.growth {
            picker = picker.growth(growth);
        }
        picker.show(ui);
        self.point.set(point);
    }
}

/// A settled fixture showing `picker`, which the caller keeps to read the point back.
fn settled(picker: &Rc<Picker>) -> Fixture {
    let shown = picker.clone();
    let mut fixture = Fixture::new(move |ui| shown.show(ui));
    fixture.settle();
    fixture
}

/// The nine points' cells, as the picker reports them to a screen reader: also the names and the count it keeps.
fn cells(fixture: &Fixture) -> Vec<Rect> {
    NAMES.iter().map(|name| fixture.by_label(name).unwrap_or_else(|| panic!("no `{name}` point")).rect).collect()
}

/// The mark a point shows: the shape drawn inside its cell that is smaller than the cell and as tall as it is wide.
fn mark<'a>(shapes: &'a [Shape], cell: Rect) -> Option<&'a RectShape> {
    shapes.iter().find_map(|shape| match shape {
        Shape::Rect(rect)
            if cell.contains_rect(rect.rect) && rect.rect.width() < cell.width() && (rect.rect.width() - rect.rect.height()).abs() <= ROUNDING =>
        {
            Some(rect)
        }
        _ => None,
    })
}

/// The mark of each of the nine points, in the order of [`NAMES`].
fn marks<'a>(shapes: &'a [Shape], cells: &[Rect]) -> Vec<&'a RectShape> {
    cells.iter().map(|cell| mark(shapes, *cell).unwrap_or_else(|| panic!("no mark in the cell at {:?}", cell.center()))).collect()
}

/// The lines a frame painted, as the two points each joins.
fn lines(shapes: &[Shape]) -> Vec<[Pos2; 2]> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, .. } => Some(*points),
            Shape::Path(path) if path.points.len() == 2 => Some([path.points[0], path.points[1]]),
            _ => None,
        })
        .collect()
}

/// The lines the squares presentation draws: the point of each cell joined to the point on its right and to the one
/// below it, from one square's edge to the other's.
fn links(marks: &[&RectShape]) -> Vec<[Pos2; 2]> {
    let mut links = Vec::new();
    for (at, mark) in marks.iter().enumerate() {
        let (column, row) = (at % GRID, at / GRID);
        if column + 1 < GRID {
            links.push([mark.rect.right_center(), marks[at + 1].rect.left_center()]);
        }
        if row + 1 < GRID {
            links.push([mark.rect.center_bottom(), marks[at + GRID].rect.center_top()]);
        }
    }
    links
}

/// The lines the grid joins its squares with: two per row and two per column.
const GRID_LINES: usize = 2 * GRID * (GRID - 1);

fn same(first: Pos2, second: Pos2) -> bool {
    (first - second).length() <= ROUNDING
}

/// Whether `painted` holds a line joining the two points `wanted` joins, either way round.
fn joins(painted: &[[Pos2; 2]], wanted: [Pos2; 2]) -> bool {
    painted.iter().any(|line| (same(line[0], wanted[0]) && same(line[1], wanted[1])) || (same(line[0], wanted[1]) && same(line[1], wanted[0])))
}

#[trial]
fn the_default_presentation_marks_the_nine_points_with_dots_and_joins_none_of_them() {
    let picker = Rc::new(Picker::dots());
    let mut fixture = settled(&picker);
    let cells = cells(&fixture);
    assert_eq!(cells.len(), GRID * GRID, "a cell for each point of the box");
    let shapes = fixture.frame_shapes();
    let marks = marks(&shapes, &cells);
    let painted = lines(&shapes);
    assert!(painted.is_empty(), "the default presentation joins nothing: {painted:?}");
    for (at, mark) in marks.iter().enumerate() {
        assert!(cells[at].contains_rect(mark.rect), "the dot of the point at {at} stays in its cell");
        assert!(same(mark.rect.center(), cells[at].center()), "the dot of the point at {at} sits at the centre of its cell");
        assert!(mark.fill.a() > 0, "the dot of the point at {at} is filled");
        assert!(mark.stroke.is_empty(), "the dot of the point at {at} is not outlined");
    }
    assert!(marks[CENTRE].rect.width() > marks[0].rect.width(), "the pick's dot is the wider one");
}

#[trial]
fn the_squares_presentation_outlines_the_points_and_joins_them_into_the_boxs_grid() {
    let picker = Rc::new(Picker::squares());
    let mut fixture = settled(&picker);
    let cells = cells(&fixture);
    let shapes = fixture.frame_shapes();
    let marks = marks(&shapes, &cells);
    for (at, mark) in marks.iter().enumerate() {
        assert!(cells[at].contains_rect(mark.rect), "the square of the point at {at} stays in its cell");
        assert!(same(mark.rect.center(), cells[at].center()), "the square of the point at {at} sits at the centre of its cell");
        assert_eq!(mark.corner_radius, egui::CornerRadius::ZERO, "the square of the point at {at} has square corners");
        let picked = at == CENTRE;
        assert_eq!(mark.fill.a() > 0, picked, "the pick's square is filled and the rest are not, at {at}");
        assert_eq!(!mark.stroke.is_empty(), !picked, "the rest are outlined and the pick is not, at {at}");
    }
    let painted = lines(&shapes);
    let wanted = links(&marks);
    assert_eq!(painted.len(), GRID_LINES, "the squares are joined to the neighbours they have: {painted:?}");
    assert_eq!(wanted.len(), GRID_LINES, "and the joins wanted are the same number");
    for line in &wanted {
        assert!(joins(&painted, *line), "a line joins {line:?}, of {painted:?}");
    }
}

#[trial]
fn the_presentation_changes_the_mark_and_leaves_every_cell_and_the_size_alone() {
    let dots = Rc::new(Picker::dots());
    let squares = Rc::new(Picker::squares());
    let (dot_fixture, square_fixture) = (settled(&dots), settled(&squares));
    assert_eq!(cells(&dot_fixture), cells(&square_fixture), "the nine parts and the cells they sense keep their places");
    assert_eq!(dot_fixture.shown(), square_fixture.shown(), "and the picker takes the size it took before");
    let compact_dots = Rc::new(Picker::dots().compact());
    let compact_squares = Rc::new(Picker::squares().compact());
    let (small_dots, mut small_squares) = (settled(&compact_dots), settled(&compact_squares));
    assert_eq!(small_dots.shown(), small_squares.shown(), "the compact picker keeps its icon size too");
    assert!(small_squares.shown().width() < square_fixture.shown().width(), "and is the smaller picker");
    assert_eq!(cells(&small_squares).len(), GRID * GRID, "the compact picker keeps its nine points");
    let shapes = small_squares.frame_shapes();
    assert_eq!(lines(&shapes).len(), GRID_LINES, "the compact squares are joined as the large ones are");
}

#[trial]
fn the_growth_arrows_are_shown_beside_the_pick_in_either_presentation() {
    let arrows = |picker: Picker| {
        let picker = Rc::new(picker.grown());
        let mut fixture = settled(&picker);
        lines(&fixture.frame_shapes()).len()
    };
    let neighbours = GRID * GRID - 1;
    assert_eq!(arrows(Picker::dots()), neighbours, "the dots presentation shows one arrow per point beside the centre pick");
    assert_eq!(arrows(Picker::squares()), neighbours + GRID_LINES, "the squares presentation keeps those arrows beside its joins");
}

#[trial]
fn a_press_on_a_square_picks_its_point_and_space_picks_the_focused_one() {
    let picker = Rc::new(Picker::squares());
    let mut fixture = settled(&picker);
    let corner = fixture.by_label("Bottom right").expect("the bottom right square");
    fixture.click(&corner);
    fixture.settle();
    assert_eq!(picker.point.get(), Anchor9::BottomRight, "a press on a square picks its point");
    let corner = fixture.by_label("Top left").expect("the top left square");
    fixture.access(AccessActionKind::Focus, &corner);
    fixture.settle();
    fixture.key(Key::Space, Modifiers::NONE);
    fixture.settle();
    assert_eq!(picker.point.get(), Anchor9::TopLeft, "Space picks the square the focus is on");
}
