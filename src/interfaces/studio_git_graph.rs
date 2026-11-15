//! The history graph of one commit, drawn: colored lanes, curved joins and a node.
//!
//! Each row of the list owns a canvas as tall as the row plus its expanded details, so a lane
//! keeps running down through the details instead of breaking.
use crate::features::git::types::GraphRow;
use crate::interfaces::studio::design;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

/// Height of the clickable part of a row; the node sits in the middle of it.
pub(super) const ROW_HEIGHT: f32 = 40.0;
const NODE_Y: f32 = ROW_HEIGHT / 2.0;
const PADDING: f32 = 10.0;
const MAX_WIDTH: f32 = 92.0;
const MAX_STEP: f32 = 13.0;

/// One color per lane; lane 0 is the trunk.
const LANES: [Color; 6] = [
    design::BRAND_BLUE,
    design::SUCCESS,
    design::TINT_PURPLE,
    design::TINT_ORANGE,
    design::BRAND_CYAN,
    design::TINT_YELLOW,
];

pub(super) fn lane_color(lane: usize) -> Color {
    LANES[lane % LANES.len()]
}

/// Distance between lanes: shrinks when there are many so the graph never eats the message.
pub(super) fn step(lanes: usize) -> f32 {
    if lanes <= 1 {
        return MAX_STEP;
    }
    ((MAX_WIDTH - PADDING * 2.0) / (lanes - 1) as f32).clamp(5.0, MAX_STEP)
}

pub(super) fn width(lanes: usize) -> f32 {
    PADDING * 2.0 + step(lanes) * lanes.saturating_sub(1) as f32
}

pub(super) struct Graph {
    pub row: GraphRow,
    pub lanes: usize,
    /// The commit HEAD points to: drawn bigger, with a ring.
    pub head: bool,
    pub selected: bool,
}

impl<Message> canvas::Program<Message> for Graph {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let height = bounds.height;
        let step = step(self.lanes);
        let x = |lane: usize| PADDING + step * lane as f32;
        let line = |color: Color| {
            Stroke::default()
                .with_color(color)
                .with_width(1.8)
                .with_line_cap(LineCap::Round)
        };
        if self.selected {
            frame.fill_rectangle(
                Point::ORIGIN,
                Size::new(bounds.width, ROW_HEIGHT.min(height)),
                design::BG_ACTIVE,
            );
        }
        let row = &self.row;
        for &lane in &row.through {
            frame.stroke(
                &Path::line(Point::new(x(lane), 0.0), Point::new(x(lane), height)),
                line(lane_color(lane)),
            );
        }
        let node = Point::new(x(row.lane), NODE_Y);
        if row.incoming {
            frame.stroke(
                &Path::line(Point::new(x(row.lane), 0.0), node),
                line(lane_color(row.lane)),
            );
        }
        for &lane in &row.merge_in {
            // A line from another lane bends into this node.
            let curve = Path::new(|path| {
                path.move_to(Point::new(x(lane), 0.0));
                path.bezier_curve_to(
                    Point::new(x(lane), NODE_Y * 0.9),
                    Point::new(x(row.lane), NODE_Y * 0.4),
                    node,
                );
            });
            frame.stroke(&curve, line(lane_color(lane)));
        }
        for &parent in &row.parent_lanes {
            if parent == row.lane {
                frame.stroke(
                    &Path::line(node, Point::new(x(parent), height)),
                    line(lane_color(parent)),
                );
            } else {
                // The branch leaves this node, bends to its lane and keeps going down.
                let curve = Path::new(|path| {
                    path.move_to(node);
                    path.bezier_curve_to(
                        Point::new(x(row.lane), ROW_HEIGHT * 0.78),
                        Point::new(x(parent), ROW_HEIGHT * 0.62),
                        Point::new(x(parent), ROW_HEIGHT),
                    );
                    path.line_to(Point::new(x(parent), height.max(ROW_HEIGHT)));
                });
                frame.stroke(&curve, line(lane_color(parent)));
            }
        }
        let color = lane_color(row.lane);
        if self.head {
            frame.stroke(
                &Path::circle(node, 6.5),
                Stroke::default()
                    .with_color(design::alpha(color, 0.45))
                    .with_width(1.5),
            );
        }
        // A dark halo keeps the lines from touching the dot.
        frame.fill(&Path::circle(node, 5.0), design::BG_PANEL);
        frame.fill(
            &Path::circle(node, if self.head { 4.2 } else { 3.4 }),
            color,
        );
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_graph_never_eats_the_message_however_many_lanes_there_are() {
        for lanes in 1..=40 {
            assert!(width(lanes) <= MAX_WIDTH.max(PADDING * 2.0 + 5.0 * (lanes - 1) as f32));
            assert!(step(lanes) >= 5.0 && step(lanes) <= MAX_STEP);
        }
        // Few lanes keep the comfortable spacing; many lanes squeeze together.
        assert_eq!(step(1), MAX_STEP);
        assert_eq!(step(3), MAX_STEP);
        assert!(step(12) < MAX_STEP);
        assert!(width(6) <= MAX_WIDTH);
    }

    #[test]
    fn lanes_wrap_around_the_palette_so_a_deep_graph_still_has_colors() {
        assert_eq!(lane_color(0), lane_color(LANES.len()));
        assert_ne!(lane_color(0), lane_color(1));
    }
}
