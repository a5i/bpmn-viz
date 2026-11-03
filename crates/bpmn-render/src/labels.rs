//! Label layout: positions, wrapping and rotated container labels.
//!
//! Label bounds from DI (`bpmndi:BPMNLabel`) take precedence over defaults;
//! otherwise positions follow common BPMN conventions. Wrapping is
//! deterministic (see `bpmn-scene::text`).

use crate::RenderOptions;
use crate::flows;
use bpmn_model::di::Edge;
use bpmn_model::di::Shape;
use bpmn_model::element::{FlowNode, NodeKind, SequenceFlow};
use bpmn_scene::TextAnchor;
use bpmn_scene::style::TextStyle;
use bpmn_scene::text::wrap_text;
use bpmn_scene::{Node, Scene};
use kurbo::Affine;

fn text_style(options: &RenderOptions) -> TextStyle {
    TextStyle {
        family: options.font_family.clone(),
        size: options.font_size,
        weight: bpmn_scene::style::FontWeight::Normal,
        fill: options.theme.text,
        italic: false,
    }
}

/// Emits wrapped text lines; returns the number of lines drawn.
#[allow(clippy::too_many_arguments)]
fn draw_wrapped(
    scene: &mut Scene,
    id: &str,
    text: &str,
    x: f64,
    y: f64,
    max_width: f64,
    anchor: TextAnchor,
    options: &RenderOptions,
    line_height: f64,
) -> usize {
    let lines = wrap_text(text, max_width, options.font_size);
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let ly = y + options.font_size / 2.0 + index as f64 * line_height;
        scene.push(Node::text(
            if index == 0 {
                id.to_string()
            } else {
                format!("{id}-{index}")
            },
            x,
            ly,
            line,
            text_style(options),
            anchor,
        ));
    }
    lines.len()
}

/// Label for a flow node (inside tasks, below events and gateways).
pub fn draw_node_label(scene: &mut Scene, node: &FlowNode, shape: &Shape, options: &RenderOptions) {
    let Some(name) = node.name.as_deref().filter(|name| !name.trim().is_empty()) else {
        return;
    };
    let id = format!("bpmn-label-{}", node.id);
    if let Some(bounds) = shape
        .label
        .and_then(|label| label.bounds)
        .filter(|b| b.is_valid())
    {
        draw_wrapped(
            scene,
            &id,
            name,
            bounds.x,
            bounds.y,
            bounds.width,
            TextAnchor::Start,
            options,
            options.font_size * 1.2,
        );
        return;
    }
    let line_height = options.font_size * 1.2;
    match &node.kind {
        NodeKind::Task { .. } | NodeKind::SubProcess { .. } => {
            // Centered inside the shape.
            let Some((cx, _)) = shape.bounds.center() else {
                return;
            };
            let max_width = (shape.bounds.width - 10.0).max(20.0);
            let lines = wrap_text(name, max_width, options.font_size).len() as f64;
            let block_height = lines * line_height;
            let top = shape.bounds.y + (shape.bounds.height - block_height) / 2.0;
            draw_wrapped(
                scene,
                &id,
                name,
                cx,
                top,
                max_width,
                TextAnchor::Middle,
                options,
                line_height,
            );
        }
        _ => {
            // Events and gateways: below the shape.
            let Some((cx, _)) = shape.bounds.center() else {
                return;
            };
            let y = shape.bounds.y + shape.bounds.height + 4.0;
            draw_wrapped(
                scene,
                &id,
                name,
                cx,
                y,
                120.0,
                TextAnchor::Middle,
                options,
                line_height,
            );
        }
    }
}

/// Rotated pool name inside the header band.
pub fn draw_participant_label(
    scene: &mut Scene,
    name: Option<&str>,
    shape: &Shape,
    options: &RenderOptions,
) {
    draw_container_label(scene, name, shape, options, "bpmn-label-pool");
}

/// Rotated lane name inside the lane's left strip.
pub fn draw_lane_label(
    scene: &mut Scene,
    name: Option<&str>,
    shape: &Shape,
    options: &RenderOptions,
) {
    draw_container_label(scene, name, shape, options, "bpmn-label-lane");
}

fn draw_container_label(
    scene: &mut Scene,
    name: Option<&str>,
    shape: &Shape,
    options: &RenderOptions,
    id_prefix: &str,
) {
    let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) else {
        return;
    };
    let horizontal = shape.is_horizontal.unwrap_or(true);
    let id = format!("{id_prefix}-{}", shape.bpmn_element);
    if horizontal {
        // Vertical text in the left band, centered.
        let cx = shape.bounds.x + 15.0_f64.min(shape.bounds.width / 2.0);
        let cy = shape.bounds.y + shape.bounds.height / 2.0;
        let transform = Affine::translate((cx, cy)) * Affine::rotate(-std::f64::consts::FRAC_PI_2);
        scene.wrap_next(Some(id.clone()), transform);
        scene.push(Node::text(
            format!("{id}-text"),
            0.0,
            0.0,
            name.to_string(),
            text_style(options),
            TextAnchor::Middle,
        ));
    } else {
        // Horizontal text in the top band.
        let cx = shape.bounds.x + shape.bounds.width / 2.0;
        let cy = shape.bounds.y + 15.0_f64.min(shape.bounds.height / 2.0);
        scene.push(Node::text(
            id,
            cx,
            cy + options.font_size / 2.0,
            name.to_string(),
            text_style(options),
            TextAnchor::Middle,
        ));
    }
}

/// Flow name at the polyline midpoint (or DI label bounds).
pub fn draw_flow_label(
    scene: &mut Scene,
    flow: &SequenceFlow,
    edge: &Edge,
    options: &RenderOptions,
) {
    let Some(name) = flow.name.as_deref().filter(|name| !name.trim().is_empty()) else {
        return;
    };
    let id = format!("bpmn-label-{}", flow.id);
    let line_height = options.font_size * 1.2;
    if let Some(bounds) = edge
        .label
        .and_then(|label| label.bounds)
        .filter(|b| b.is_valid())
    {
        draw_wrapped(
            scene,
            &id,
            name,
            bounds.x,
            bounds.y,
            bounds.width,
            TextAnchor::Start,
            options,
            line_height,
        );
        return;
    }
    let points: Vec<(f64, f64)> = edge.waypoints.iter().map(|p| (p.x, p.y)).collect();
    let Some((mx, my)) = flows::polyline_midpoint(&points) else {
        return;
    };
    draw_wrapped(
        scene,
        &id,
        name,
        mx,
        my - options.font_size - 2.0,
        140.0,
        TextAnchor::Middle,
        options,
        line_height,
    );
}

/// Message flow name at the polyline midpoint.
pub fn draw_message_flow_label(
    scene: &mut Scene,
    flow: &bpmn_model::element::MessageFlow,
    edge: &Edge,
    options: &RenderOptions,
) {
    let Some(name) = flow.name.as_deref().filter(|name| !name.trim().is_empty()) else {
        return;
    };
    let id = format!("bpmn-label-{}", flow.id);
    let points: Vec<(f64, f64)> = edge.waypoints.iter().map(|p| (p.x, p.y)).collect();
    let Some((mx, my)) = flows::polyline_midpoint(&points) else {
        return;
    };
    draw_wrapped(
        scene,
        &id,
        name,
        mx,
        my - options.font_size - 2.0,
        140.0,
        bpmn_scene::TextAnchor::Middle,
        options,
        options.font_size * 1.2,
    );
}

/// Annotation text right of the bracket.
pub fn draw_annotation_text(
    scene: &mut Scene,
    annotation_id: &str,
    text: &str,
    shape: &Shape,
    options: &RenderOptions,
) {
    let id = format!("bpmn-label-{}", annotation_id);
    draw_wrapped(
        scene,
        &id,
        text,
        shape.bounds.x + 12.0,
        shape.bounds.y + 4.0,
        (shape.bounds.width - 14.0).max(20.0),
        TextAnchor::Start,
        options,
        options.font_size * 1.2,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_model::di::Bounds;
    use bpmn_model::element::{FlowNode, TaskType};
    use bpmn_scene::style::Viewport;

    fn task_shape() -> Shape {
        Shape {
            id: Some("s".into()),
            bpmn_element: "T".into(),
            bounds: Bounds::new(0.0, 0.0, 100.0, 80.0),
            label: None,
            is_horizontal: None,
            is_expanded: None,
        }
    }

    fn node(name: Option<&str>) -> FlowNode {
        FlowNode {
            id: "T".into(),
            name: name.map(str::to_string),
            incoming: Vec::new(),
            outgoing: Vec::new(),
            default_flow_ref: None,
            kind: NodeKind::Task {
                task_type: TaskType::Abstract,
            },
        }
    }

    #[test]
    fn task_label_is_centered_and_wrapped() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 120.0, 100.0));
        let options = RenderOptions::default();
        draw_node_label(
            &mut scene,
            &node(Some("approve request with budget check")),
            &task_shape(),
            &options,
        );
        assert!(scene.nodes.len() >= 2);
        for candidate in &scene.nodes {
            if let Node::Text { anchor, .. } = candidate {
                assert_eq!(*anchor, TextAnchor::Middle);
            }
        }
    }

    #[test]
    fn unnamed_node_has_no_label() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 120.0, 100.0));
        draw_node_label(
            &mut scene,
            &node(None),
            &task_shape(),
            &RenderOptions::default(),
        );
        assert!(scene.nodes.is_empty());
    }

    #[test]
    fn label_bounds_take_precedence() {
        let mut shape = task_shape();
        shape.label = Some(bpmn_model::di::BpmnLabel {
            bounds: Some(Bounds::new(200.0, 10.0, 60.0, 30.0)),
        });
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 320.0, 100.0));
        draw_node_label(
            &mut scene,
            &node(Some("hi")),
            &shape,
            &RenderOptions::default(),
        );
        match &scene.nodes[0] {
            Node::Text { x, y, .. } => {
                assert_eq!(*x, 200.0);
                assert!(*y > 10.0);
            }
            other => panic!("expected text, got {other:?}"),
        }
    }
}
