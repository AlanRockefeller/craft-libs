//! `ReferencePoint` marks its nine points with the dot the picker has always shown; asked for a presentation, it marks
//! them with the squares of the box's grid, joined to their neighbours, and keeps every part, cell, name and pick. The
//! squares take their share of the cell they are drawn in, so they and their joins fit even a room smaller than the
//! picker asks for.

use craft_behavior::Anchor9;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::pickers::{PointPresentation, ReferencePoint};
use craft_ui::testing::Fixture;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::trial;
use egui::epaint::RectShape;
use egui::{Color32, Key, Modifiers, Pos2, Rect, Shape, Ui, Vec2};
use std::cell::Cell;
use std::rc::Rc;

/// Three points on each side of the picker's grid.
const GRID: usize = 3;

/// The centre point's place among the nine, which the fixtures pick.
const CENTRE: usize = 4;

/// The slack a painted shape is compared with: the rounding of the layout.
const ROUNDING: f32 = 0.5;

/// The share of its cell a square takes, as the presentation's own constant holds it: the look the observed proxy gives
/// the nine points, with about a third of the cell left for the line that joins them.
const SQUARE_OF_CELL: f32 = 0.7;

/// The slack the share is read with, a twentieth of it: a rect the layout rounded moves a share this much at most.
const SHARE_SLACK: f32 = 0.05;

/// The nine points' names, in the order the picker declares them: row by row from the top left.
const NAMES: [&str; GRID * GRID] = ["Top left", "Top", "Top right", "Left", "Center", "Right", "Bottom left", "Bottom", "Bottom right"];

/// The picker a trial shows, holding the point it edits and the mark it shows it with.
struct Picker {
    point: Cell<Anchor9>,
    presentation: PointPresentation,
    compact: bool,
    growth: Option<(f64, f64)>,
    enabled: bool,
}

impl Picker {
    /// The picker as the library shows it, the centre picked.
    fn dots() -> Self {
        Picker { point: Cell::new(Anchor9::Centre), presentation: PointPresentation::Dots, compact: false, growth: None, enabled: true }
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

    /// The picker the tokens paint disabled, so a trial can compare the two presentations' inks in that state too.
    fn disabled(self) -> Self {
        Picker { enabled: false, ..self }
    }

    /// The picker this holds, over `point`.
    fn build<'b>(&self, point: &'b mut Anchor9) -> ReferencePoint<'b> {
        let mut picker = ReferencePoint::new("Reference point", Bind::bound(point)).presentation(self.presentation).enabled(self.enabled);
        if self.compact {
            picker = picker.compact();
        }
        if let Some(growth) = self.growth {
            picker = picker.growth(growth);
        }
        picker
    }

    fn show(&self, ui: &mut Ui) {
        let mut point = self.point.get();
        self.build(&mut point).show(ui);
        self.point.set(point);
    }

    /// The picker at `rect`, the room a fixed slot gives it, rather than the size it asks for.
    fn show_in(&self, ui: &mut Ui, rect: Rect) {
        let mut point = self.point.get();
        self.build(&mut point).show_at(ui, rect);
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

/// A settled fixture showing `picker` at `rect`, which the caller keeps to read the point back.
fn settled_in(picker: &Rc<Picker>, rect: Rect) -> Fixture {
    let shown = picker.clone();
    let mut fixture = Fixture::new(move |ui| shown.show_in(ui, rect));
    fixture.settle();
    fixture
}

/// The share of the room the picker asks for that a fixed slot leaves it, in the trial that forces a smaller room.
const SLOT: f32 = 0.5;

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

/// The mark of the point in `cell`, whatever its size: the shape as tall as it is wide centred on the cell.
fn centred_mark<'a>(shapes: &'a [Shape], cell: Rect) -> Option<&'a RectShape> {
    shapes.iter().find_map(|shape| match shape {
        Shape::Rect(rect) if same(rect.rect.center(), cell.center()) && (rect.rect.width() - rect.rect.height()).abs() <= ROUNDING => Some(rect),
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

/// The middle of the vertices of a filled polygon's mesh: where the shape it fills sits.
fn middle_of(mesh: &egui::epaint::Mesh) -> Pos2 {
    let count = mesh.vertices.len().max(1) as f32;
    let sum = mesh.vertices.iter().fold(Vec2::ZERO, |sum, vertex| sum + vertex.pos.to_vec2());
    (sum / count).to_pos2()
}

/// The middles of the meshes a frame painted. At this pin the only mesh a picker's frame holds is an arrow's head:
/// `Draw::fill` of a `PaintShape::Polygon` adds `fill_polygon`'s mesh and returns before its closed path
/// (`ui-contract/src/primitives/draw.rs`), while the marks and the joins are rects and two-point paths, and the sunken
/// surface is a fill with a border stroke rather than a bevel (the suite's default theme, `craft.artcraft`, sets no
/// `shape.edge`, so the edge is flat). Nothing here filters on size, so an oversized mark cannot hide from the
/// containment assertions the trials make.
fn head_middles(shapes: &[Shape]) -> Vec<Pos2> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Mesh(mesh) => Some(middle_of(mesh)),
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

/// The ink `picker` paints its pick with and the ink of the eight points beside it: a mark that is filled gives its
/// fill, and one that is outlined gives its outline, so the two presentations are compared on the colour each point
/// reads in rather than on how it is drawn.
fn painted_inks(picker: Picker) -> (Color32, Vec<Color32>) {
    let picker = Rc::new(picker);
    let mut fixture = settled(&picker);
    let cells = cells(&fixture);
    let shapes = fixture.frame_shapes();
    let marks = marks(&shapes, &cells);
    let pick = marks[CENTRE].fill;
    let beside = marks
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != CENTRE)
        .map(|(_, mark)| if mark.stroke.is_empty() { mark.fill } else { mark.stroke.color })
        .collect();
    (pick, beside)
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

/// The look's proportion where the style's side and the drawn cell are the same: a square takes [`SQUARE_OF_CELL`] of
/// its cell at the picker's natural allocation and at the compact one. A room larger than the picker asks for caps the
/// square at the style's side for the natural size, so the share there is smaller than this: the cap is what keeps the
/// look at its design size, and the forced-room trial covers the other side.
#[trial]
fn the_square_takes_the_looks_share_of_its_cell_at_the_natural_size_and_when_compact() {
    for (name, picker) in [("full", Picker::squares()), ("compact", Picker::squares().compact())] {
        let picker = Rc::new(picker);
        let mut fixture = settled(&picker);
        let cells = cells(&fixture);
        let shapes = fixture.frame_shapes();
        let marks = marks(&shapes, &cells);
        for (at, mark) in marks.iter().enumerate() {
            let share = mark.rect.width() / cells[at].width();
            assert!(
                (share - SQUARE_OF_CELL).abs() <= SHARE_SLACK,
                "the {name} square at {at} takes {share} of its cell, not the {SQUARE_OF_CELL} the presentation holds"
            );
        }
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
fn the_growth_arrows_keep_their_shafts_beside_the_pick_in_either_presentation() {
    let shafts = |picker: Picker| {
        let picker = Rc::new(picker.grown());
        let mut fixture = settled(&picker);
        lines(&fixture.frame_shapes()).len()
    };
    let neighbours = GRID * GRID - 1;
    assert_eq!(shafts(Picker::dots()), neighbours, "the dots presentation shows one shaft per point beside the centre pick");
    assert_eq!(shafts(Picker::squares()), neighbours + GRID_LINES, "the squares presentation keeps those shafts beside its twelve joins");
}

#[trial]
fn the_only_meshes_a_picker_paints_are_the_heads_of_its_growth_arrows() {
    let neighbours = GRID * GRID - 1;
    for (name, squares) in [("dots", false), ("squares", true)] {
        for grown in [false, true] {
            let mut picker = Picker::dots();
            if squares {
                picker = Picker::squares();
            }
            if grown {
                picker = picker.grown();
            }
            let picker = Rc::new(picker);
            let mut fixture = settled(&picker);
            let cells = cells(&fixture);
            let middles = head_middles(&fixture.frame_shapes());
            let wanted = if grown { neighbours } else { 0 };
            assert_eq!(middles.len(), wanted, "the {name} presentation paints {wanted} meshes with growth: {middles:?}");
            for at in (0..GRID * GRID).filter(|at| *at != CENTRE) {
                assert_eq!(
                    middles.iter().filter(|middle| cells[at].contains(**middle)).count(),
                    if grown { 1 } else { 0 },
                    "the cell at {at} holds the middle of the head beside the pick, {name}"
                );
            }
            if grown {
                assert!(middles.iter().all(|middle| !cells[CENTRE].contains(*middle)), "and no head's middle sits in the pick's own cell, {name}");
            }
        }
    }
}

/// The squares read in the dots' inks. The two states say different things: enabled, the pick's ink is the one the
/// pick's dot reads in and the eight others read in theirs, which are two different inks, so the comparison tells a
/// pick's ink from a point's; disabled, the tokens give the pick and the points one colour, so that pass shows the two
/// presentations agree on the state rather than that they tell the inks apart.
#[trial]
fn the_squares_take_the_inks_of_the_dots_they_stand_in_for() {
    let states = [("enabled", true, Picker::dots(), Picker::squares()), ("disabled", false, Picker::dots().disabled(), Picker::squares().disabled())];
    for (state, apart, dots, squares) in states {
        let (dot_pick, dot_beside) = painted_inks(dots);
        let (square_pick, square_beside) = painted_inks(squares);
        if apart {
            assert!(
                dot_beside.iter().all(|ink| *ink != dot_pick),
                "in the enabled state the pick reads in an ink of its own, so this comparison tells the two apart"
            );
        }
        assert_eq!(square_pick, dot_pick, "in the {state} state the pick's square is filled with the ink of the pick's dot");
        assert_eq!(square_beside, dot_beside, "and the other squares are outlined in the inks of the other dots, in the {state} state");
    }
}

#[trial]
fn the_squares_and_their_joins_fit_a_room_smaller_than_the_picker_asks_for() {
    let picker = Rc::new(Picker::squares());
    let mut natural = settled(&picker);
    let asked = natural.shown();
    let natural_side = {
        let shapes = natural.frame_shapes();
        marks(&shapes, &cells(&natural))[CENTRE].rect.width()
    };
    let forced = Rect::from_min_size(asked.min, asked.size() * SLOT);
    let mut fixture = settled_in(&picker, forced);
    let cells = cells(&fixture);
    assert_eq!(cells.len(), GRID * GRID, "the fixed room keeps the nine points");
    assert!(
        cells[0].width() < asked.width() / GRID as f32,
        "the room is smaller than the one the picker asks for: a cell of {:?} in a room of {forced:?}",
        cells[0].size()
    );
    let shapes = fixture.frame_shapes();
    let marks: Vec<&RectShape> = cells
        .iter()
        .enumerate()
        .map(|(at, cell)| centred_mark(&shapes, *cell).unwrap_or_else(|| panic!("no mark at the centre of the cell at {at}")))
        .collect();
    for (at, (cell, mark)) in cells.iter().zip(&marks).enumerate() {
        assert!(cell.contains_rect(mark.rect), "the square of the point at {at} stays in its cell: {:?} in {cell:?}", mark.rect);
    }
    assert!(marks[0].rect.width() < natural_side, "the squares shrink with the room: {:?} against {natural_side}", marks[0].rect.width());
    let painted = lines(&shapes);
    let wanted = links(&marks);
    assert_eq!(painted.len(), GRID_LINES, "the squares are still joined to the neighbours they have: {painted:?}");
    assert_eq!(wanted.len(), GRID_LINES, "and the joins wanted are the same number");
    for line in &wanted {
        assert!(joins(&painted, *line), "a line joins {line:?}, of {painted:?}");
    }
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
