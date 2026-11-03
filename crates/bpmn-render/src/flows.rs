//! Sequence flow edges: waypoint polylines, markers and decorations.

use crate::RenderOptions;
use bpmn_model::di::Edge;
use bpmn_model::element::{MessageFlow, SequenceFlow};
use bpmn_scene::path;
use bpmn_scene::style::{Paint, Style};
use bpmn_scene::{Node, Scene, ids};
use kurbo::BezPath;

fn flow_style(options: &RenderOptions, marker_end: bool) -> Style {
    Style {
        fill: Some(Paint::None),
        stroke: Some(Paint::Color(options.theme.flow_stroke)),
        stroke_width: 1.5,
        dash_array: None,
        marker_end: marker_end.then(|| ids::ARROW_END.to_string()),
        marker_start: None,
    }
}

/// Draws a sequence flow polyline with an arrowhead. Default flows get a
/// slash tick near the start; conditional flows get the conditional diamond.
pub fn draw_flow(scene: &mut Scene, flow: &SequenceFlow, edge: &Edge, options: &RenderOptions) {
    if edge.waypoints.len() < 2 {
        return; // validated elsewhere; never invent geometry
    }
    let points: Vec<(f64, f64)> = edge.waypoints.iter().map(|p| (p.x, p.y)).collect();
    let id = format!("bpmn-edge-{}", flow.id);
    let mut style = flow_style(options, true);
    if flow.condition.is_some() && !flow.is_default {
        style.marker_start = Some(ids::CONDITIONAL_MARKER.to_string());
    }
    scene.push(Node::path(id, path::polyline(&points), style));

    if flow.is_default {
        // Slash tick drawn a few units after the first waypoint.
        let (x0, y0) = points[0];
        let (x1, y1) = points[1];
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1e-9 {
            let ux = dx / len;
            let uy = dy / len;
            let t = 8.0_f64.min(len / 2.0);
            let cx = x0 + ux * t;
            let cy = y0 + uy * t;
            let mut tick = BezPath::new();
            tick.move_to((cx - uy * 4.0, cy + ux * 4.0));
            tick.line_to((cx + ux * 4.0, cy - uy * 4.0));
            let mut tick_style = flow_style(options, false);
            tick_style.stroke_width = 1.8;
            scene.push(Node::path(
                format!("bpmn-edge-{}-default", flow.id),
                tick,
                tick_style,
            ));
        }
    }
}

/// Draws a message flow: dashed line without a sequence arrowhead.
pub fn draw_message_flow(
    scene: &mut Scene,
    flow: &MessageFlow,
    edge: &Edge,
    options: &RenderOptions,
) {
    if edge.waypoints.len() < 2 {
        return; // never invent geometry
    }
    let points: Vec<(f64, f64)> = edge.waypoints.iter().map(|p| (p.x, p.y)).collect();
    let mut style = flow_style(options, false);
    style.dash_array = Some(vec![6.0, 4.0]);
    scene.push(Node::path(
        format!("bpmn-edge-{}", flow.id),
        path::polyline(&points),
        style,
    ));
}

/// Midpoint of the polyline by segment length, for label placement.
pub fn polyline_midpoint(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    if points.len() < 2 {
        return None;
    }
    let mut total = 0.0;
    for pair in points.windows(2) {
        let dx = pair[1].0 - pair[0].0;
        let dy = pair[1].1 - pair[0].1;
        total += (dx * dx + dy * dy).sqrt();
    }
    if total == 0.0 {
        return Some(points[0]);
    }
    let mut walked = 0.0;
    for pair in points.windows(2) {
        let dx = pair[1].0 - pair[0].0;
        let dy = pair[1].1 - pair[0].1;
        let segment = (dx * dx + dy * dy).sqrt();
        if walked + segment >= total / 2.0 {
            let t = if segment == 0.0 {
                0.0
            } else {
                (total / 2.0 - walked) / segment
            };
            return Some((pair[0].0 + dx * t, pair[0].1 + dy * t));
        }
        walked += segment;
    }
    Some(*points.last().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_model::di::Point;
    use bpmn_scene::style::Viewport;

    fn edge(waypoints: &[(f64, f64)]) -> Edge {
        Edge {
            id: Some("e".into()),
            bpmn_element: "F".into(),
            waypoints: waypoints.iter().map(|(x, y)| Point::new(*x, *y)).collect(),
            label: None,
        }
    }

    fn flow() -> SequenceFlow {
        SequenceFlow {
            id: "F".into(),
            name: Some("ok".into()),
            source_ref: "A".into(),
            target_ref: "B".into(),
            is_default: false,
            condition: None,
        }
    }

    #[test]
    fn flow_without_waypoints_draws_nothing() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 10.0, 10.0));
        draw_flow(&mut scene, &flow(), &edge(&[]), &RenderOptions::default());
        assert!(scene.nodes.is_empty());
    }

    #[test]
    fn flow_draws_polyline_with_arrow() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 100.0, 10.0));
        draw_flow(
            &mut scene,
            &flow(),
            &edge(&[(0.0, 5.0), (100.0, 5.0)]),
            &RenderOptions::default(),
        );
        assert_eq!(scene.nodes.len(), 1);
        match &scene.nodes[0] {
            Node::Path { style, .. } => {
                assert_eq!(style.marker_end.as_deref(), Some(ids::ARROW_END));
            }
            other => panic!("expected path, got {other:?}"),
        }
    }

    #[test]
    fn default_flow_gets_tick_and_conditional_gets_marker() {
        let options = RenderOptions::default();
        let mut default_flow = flow();
        default_flow.is_default = true;
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 100.0, 10.0));
        draw_flow(
            &mut scene,
            &default_flow,
            &edge(&[(0.0, 5.0), (100.0, 5.0)]),
            &options,
        );
        assert_eq!(scene.nodes.len(), 2);

        let mut conditional = flow();
        conditional.condition = Some("x > 1".into());
        let mut scene2 = Scene::new(Viewport::new(0.0, 0.0, 100.0, 10.0));
        draw_flow(
            &mut scene2,
            &conditional,
            &edge(&[(0.0, 5.0), (100.0, 5.0)]),
            &options,
        );
        match &scene2.nodes[0] {
            Node::Path { style, .. } => {
                assert_eq!(style.marker_start.as_deref(), Some(ids::CONDITIONAL_MARKER));
            }
            other => panic!("expected path, got {other:?}"),
        }
    }

    #[test]
    fn midpoint_of_two_segments() {
        // Total length 20, half = 10 → exactly the corner.
        let mid = polyline_midpoint(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        assert_eq!(mid, Some((10.0, 0.0)));
        // Total length 30, half = 15 → 15 units along the first leg.
        let mid = polyline_midpoint(&[(0.0, 0.0), (20.0, 0.0), (20.0, 10.0)]);
        assert_eq!(mid, Some((15.0, 0.0)));
    }
}
