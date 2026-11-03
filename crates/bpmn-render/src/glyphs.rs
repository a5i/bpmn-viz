//! BPMN node glyphs: events, tasks, gateways, subprocesses, containers,
//! text annotations and the unsupported-symbol fallback.

use crate::RenderOptions;
use bpmn_model::di::Shape;
use bpmn_model::diagnostic::{Diagnostic, DiagnosticCode};
use bpmn_model::element::{
    EventDefinition, FlowNode, GatewayType, NodeKind, TaskType, TextAnnotation,
};
use bpmn_scene::Node;
use bpmn_scene::Scene;
use bpmn_scene::path;
use bpmn_scene::style::{Paint, Style};
use kurbo::BezPath;

fn shape_style(options: &RenderOptions, stroke_width: f64) -> Style {
    Style {
        fill: Some(Paint::Color(options.theme.shape_fill)),
        stroke: Some(Paint::Color(options.theme.shape_stroke)),
        stroke_width,
        dash_array: None,
        marker_end: None,
        marker_start: None,
    }
}

fn line_style(options: &RenderOptions, stroke_width: f64) -> Style {
    Style {
        fill: Some(Paint::None),
        stroke: Some(Paint::Color(options.theme.shape_stroke)),
        stroke_width,
        dash_array: None,
        marker_end: None,
        marker_start: None,
    }
}

fn filled_style(options: &RenderOptions) -> Style {
    Style {
        fill: Some(Paint::Color(options.theme.shape_stroke)),
        stroke: Some(Paint::None),
        stroke_width: 0.0,
        dash_array: None,
        marker_end: None,
        marker_start: None,
    }
}

/// Draws a pool participant band (geometry from DI).
pub fn draw_participant(scene: &mut Scene, shape: &Shape, options: &RenderOptions) {
    let style = shape_style(options, 1.5);
    scene.push(Node::rect(
        format!("bpmn-shape-{}", shape.bpmn_element),
        shape.bounds.x,
        shape.bounds.y,
        shape.bounds.width,
        shape.bounds.height,
        style,
    ));
    // Header band: left for horizontal pools, top otherwise.
    let horizontal = shape.is_horizontal.unwrap_or(true);
    let header_width = 30.0_f64.min(shape.bounds.width);
    let header_height = 30.0_f64.min(shape.bounds.height);
    let (x, y, w, h) = if horizontal {
        (
            shape.bounds.x,
            shape.bounds.y,
            header_width,
            shape.bounds.height,
        )
    } else {
        (
            shape.bounds.x,
            shape.bounds.y,
            shape.bounds.width,
            header_height,
        )
    };
    scene.push(Node::rect(
        format!("bpmn-shape-{}-band", shape.bpmn_element),
        x,
        y,
        w,
        h,
        shape_style(options, 1.5),
    ));
}

/// Draws a lane rectangle (geometry from DI).
pub fn draw_lane(scene: &mut Scene, shape: &Shape, options: &RenderOptions) {
    scene.push(Node::rect(
        format!("bpmn-shape-{}", shape.bpmn_element),
        shape.bounds.x,
        shape.bounds.y,
        shape.bounds.width,
        shape.bounds.height,
        shape_style(options, 1.0),
    ));
}

/// Draws one flow node glyph, emitting `symbol.unsupported` warnings for
/// kinds without a dedicated P0 glyph.
pub fn draw_node(
    scene: &mut Scene,
    node: &FlowNode,
    shape: &Shape,
    options: &RenderOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let id = format!("bpmn-shape-{}", node.id);
    match &node.kind {
        NodeKind::StartEvent { definition } => {
            draw_event(scene, &id, shape, options, false, false, *definition);
        }
        NodeKind::BoundaryEvent { definition, .. } => {
            draw_event(scene, &id, shape, options, false, true, *definition);
        }
        NodeKind::EndEvent { definition } => {
            draw_event(scene, &id, shape, options, true, false, *definition);
        }
        NodeKind::IntermediateCatchEvent { definition }
        | NodeKind::IntermediateThrowEvent { definition } => {
            draw_event(scene, &id, shape, options, false, true, *definition);
        }
        NodeKind::Task { task_type } => draw_task(scene, &id, shape, options, *task_type),
        NodeKind::Gateway { gateway_type } => {
            draw_gateway(scene, &id, shape, options, *gateway_type, diagnostics);
        }
        NodeKind::SubProcess { .. } => draw_subprocess(scene, &id, shape, options),
        NodeKind::Other { local_name } => {
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::UnsupportedSymbol,
                    format!(
                        "`{local_name}` has no dedicated glyph in this prototype; drawing fallback"
                    ),
                )
                .with_element_id(node.id.clone()),
            );
            draw_fallback(scene, &id, shape, options);
        }
    }
}

fn draw_event(
    scene: &mut Scene,
    id: &str,
    shape: &Shape,
    options: &RenderOptions,
    is_end: bool,
    is_intermediate: bool,
    definition: EventDefinition,
) {
    let Some((cx, cy)) = shape.bounds.center() else {
        return;
    };
    let radius = shape.bounds.width.min(shape.bounds.height) / 2.0;
    let stroke_width = if is_end { 3.5 } else { 1.5 };
    scene.push(Node::circle(
        id,
        cx,
        cy,
        radius,
        shape_style(options, stroke_width),
    ));
    if is_intermediate {
        scene.push(Node::circle(
            format!("{id}-inner"),
            cx,
            cy,
            (radius - 3.0).max(1.0),
            shape_style(options, 1.5),
        ));
    }
    draw_event_marker(scene, id, cx, cy, radius, definition, is_end, options);
}

#[allow(clippy::too_many_arguments)]
fn draw_event_marker(
    scene: &mut Scene,
    id: &str,
    cx: f64,
    cy: f64,
    radius: f64,
    definition: EventDefinition,
    is_end: bool,
    options: &RenderOptions,
) {
    let marker_size = radius * 0.9;
    match definition {
        EventDefinition::None => {}
        EventDefinition::Message => {
            // Envelope: rect + flap.
            let w = marker_size * 1.2;
            let h = marker_size * 0.85;
            let x = cx - w / 2.0;
            let y = cy - h / 2.0;
            scene.push(Node::rect(
                format!("{id}-marker"),
                x,
                y,
                w,
                h,
                line_style(options, 1.2),
            ));
            let mut flap = BezPath::new();
            flap.move_to((x, y));
            flap.line_to((cx, y + h * 0.55));
            flap.line_to((x + w, y));
            scene.push(Node::path(
                format!("{id}-marker-flap"),
                flap,
                line_style(options, 1.2),
            ));
        }
        EventDefinition::Timer => {
            scene.push(Node::circle(
                format!("{id}-marker"),
                cx,
                cy,
                marker_size * 0.75,
                line_style(options, 1.2),
            ));
            let mut hands = BezPath::new();
            hands.move_to((cx, cy));
            hands.line_to((cx, cy - marker_size * 0.6));
            hands.move_to((cx, cy));
            hands.line_to((cx + marker_size * 0.45, cy + marker_size * 0.2));
            scene.push(Node::path(
                format!("{id}-marker-hands"),
                hands,
                line_style(options, 1.2),
            ));
        }
        EventDefinition::Error => {
            // Lightning bolt.
            let s = marker_size;
            let bolt = path::polygon(&[
                (cx - s * 0.3, cy - s * 0.7),
                (cx + s * 0.45, cy - s * 0.7),
                (cx - s * 0.05, cy - s * 0.05),
                (cx + s * 0.35, cy - s * 0.05),
                (cx - s * 0.35, cy + s * 0.7),
                (cx - s * 0.1, cy + s * 0.05),
                (cx - s * 0.45, cy + s * 0.05),
            ]);
            scene.push(Node::path(
                format!("{id}-marker"),
                bolt,
                filled_style(options),
            ));
        }
        EventDefinition::Escalation => {
            // Ascending arrow.
            let s = marker_size;
            let arrow = path::polygon(&[
                (cx, cy - s * 0.75),
                (cx + s * 0.5, cy + s * 0.55),
                (cx, cy + s * 0.25),
                (cx - s * 0.5, cy + s * 0.55),
            ]);
            scene.push(Node::path(
                format!("{id}-marker"),
                arrow,
                filled_style(options),
            ));
        }
        EventDefinition::Terminate => {
            scene.push(Node::circle(
                format!("{id}-marker"),
                cx,
                cy,
                marker_size * 0.7,
                filled_style(options),
            ));
        }
        EventDefinition::Compensation => {
            // Two filled left arrows.
            let s = marker_size;
            for offset in [-0.45_f64, 0.05] {
                let arrow = path::polygon(&[
                    (cx + offset * s + s * 0.4, cy - s * 0.4),
                    (cx + offset * s - s * 0.35, cy),
                    (cx + offset * s + s * 0.4, cy + s * 0.4),
                    (cx + offset * s + s * 0.4, cy + s * 0.18),
                    (cx + offset * s - s * 0.05, cy),
                    (cx + offset * s + s * 0.4, cy - s * 0.18),
                ]);
                scene.push(Node::path(
                    format!("{id}-marker-{offset}"),
                    arrow,
                    filled_style(options),
                ));
            }
        }
        EventDefinition::Conditional => {
            let w = marker_size;
            let h = marker_size * 1.1;
            let x = cx - w / 2.0;
            let y = cy - h / 2.0;
            scene.push(Node::rect(
                format!("{id}-marker"),
                x,
                y,
                w,
                h,
                line_style(options, 1.2),
            ));
            for i in 0..3 {
                let mut line = BezPath::new();
                let lx = x + w * (0.2 + 0.25 * i as f64);
                line.move_to((lx, y));
                line.line_to((lx, y + h));
                scene.push(Node::path(
                    format!("{id}-marker-l{i}"),
                    line,
                    line_style(options, 1.0),
                ));
            }
        }
        EventDefinition::Link => {
            scene.push(Node::circle(
                format!("{id}-marker"),
                cx,
                cy,
                marker_size * 0.75,
                line_style(options, 1.2),
            ));
            let mut arrow = BezPath::new();
            arrow.move_to((cx - marker_size * 0.3, cy));
            arrow.line_to((cx + marker_size * 0.5, cy));
            arrow.line_to((cx + marker_size * 0.2, cy - marker_size * 0.3));
            arrow.move_to((cx + marker_size * 0.5, cy));
            arrow.line_to((cx + marker_size * 0.2, cy + marker_size * 0.3));
            scene.push(Node::path(
                format!("{id}-marker-arrow"),
                arrow,
                line_style(options, 1.2),
            ));
        }
        EventDefinition::Signal => {
            let s = marker_size;
            let triangle = path::polygon(&[
                (cx, cy - s * 0.7),
                (cx + s * 0.7, cy + s * 0.55),
                (cx - s * 0.7, cy + s * 0.55),
            ]);
            scene.push(Node::path(
                format!("{id}-marker"),
                triangle,
                filled_style(options),
            ));
        }
        EventDefinition::Cancel => {
            let s = marker_size * 0.55;
            let mut cross = BezPath::new();
            cross.move_to((cx - s, cy - s));
            cross.line_to((cx + s, cy + s));
            cross.move_to((cx + s, cy - s));
            cross.line_to((cx - s, cy + s));
            scene.push(Node::path(
                format!("{id}-marker"),
                cross,
                line_style(options, 1.4),
            ));
        }
        EventDefinition::Multiple => {
            let r = marker_size * 0.22;
            for (dx, dy) in [(-0.4, 0.0), (0.0, 0.0), (0.4, 0.0)] {
                scene.push(Node::circle(
                    format!("{id}-marker-{dx}"),
                    cx + dx * marker_size,
                    cy + dy,
                    r,
                    filled_style(options),
                ));
            }
        }
    }
    let _ = is_end;
}

fn draw_task(
    scene: &mut Scene,
    id: &str,
    shape: &Shape,
    options: &RenderOptions,
    task_type: TaskType,
) {
    scene.push(Node::rounded_rect(
        id,
        shape.bounds.x,
        shape.bounds.y,
        shape.bounds.width,
        shape.bounds.height,
        8.0,
        shape_style(options, 1.5),
    ));
    draw_task_icon(scene, id, shape, options, task_type);
}

fn draw_task_icon(
    scene: &mut Scene,
    id: &str,
    shape: &Shape,
    options: &RenderOptions,
    task_type: TaskType,
) {
    let icon_cx = shape.bounds.x + 14.0;
    let icon_cy = shape.bounds.y + 12.0;
    let icon_id = format!("{id}-icon");
    match task_type {
        TaskType::Abstract => {}
        TaskType::User => {
            // Person: head + shoulders.
            scene.push(Node::circle(
                format!("{icon_id}-head"),
                icon_cx,
                icon_cy - 3.0,
                2.6,
                line_style(options, 1.2),
            ));
            let mut body = BezPath::new();
            body.move_to((icon_cx - 5.0, icon_cy + 7.0));
            body.curve_to(
                (icon_cx - 5.0, icon_cy + 1.0),
                (icon_cx + 5.0, icon_cy + 1.0),
                (icon_cx + 5.0, icon_cy + 7.0),
            );
            scene.push(Node::path(icon_id, body, line_style(options, 1.2)));
        }
        TaskType::Service => {
            // Gear: circle plus spokes.
            scene.push(Node::circle(
                format!("{icon_id}-hub"),
                icon_cx,
                icon_cy,
                3.2,
                line_style(options, 1.2),
            ));
            let mut spokes = BezPath::new();
            for i in 0..8 {
                let angle = i as f64 * std::f64::consts::FRAC_PI_4;
                let (dx, dy) = (angle.cos(), angle.sin());
                spokes.move_to((icon_cx + dx * 4.4, icon_cy + dy * 4.4));
                spokes.line_to((icon_cx + dx * 6.4, icon_cy + dy * 6.4));
            }
            scene.push(Node::path(icon_id, spokes, line_style(options, 1.2)));
        }
        TaskType::Script => {
            // Document with lines.
            let mut doc = BezPath::new();
            doc.move_to((icon_cx - 4.0, icon_cy - 6.0));
            doc.line_to((icon_cx + 4.5, icon_cy - 6.0));
            doc.line_to((icon_cx + 4.5, icon_cy + 6.5));
            doc.line_to((icon_cx - 4.0, icon_cy + 6.5));
            doc.close_path();
            scene.push(Node::path(
                format!("{icon_id}-paper"),
                doc,
                line_style(options, 1.2),
            ));
            let mut lines = BezPath::new();
            for dy in [-2.5, 0.5, 3.5] {
                lines.move_to((icon_cx - 2.0, icon_cy + dy));
                lines.line_to((icon_cx + 2.5, icon_cy + dy));
            }
            scene.push(Node::path(icon_id, lines, line_style(options, 1.0)));
        }
        TaskType::Manual => {
            // Simplified hand: palm rect + thumb notch.
            let mut hand = BezPath::new();
            hand.move_to((icon_cx - 4.5, icon_cy + 6.0));
            hand.line_to((icon_cx - 4.5, icon_cy - 1.0));
            hand.line_to((icon_cx - 2.5, icon_cy - 1.0));
            hand.line_to((icon_cx - 2.5, icon_cy - 5.5));
            hand.line_to((icon_cx - 0.8, icon_cy - 5.5));
            hand.line_to((icon_cx - 0.8, icon_cy - 1.5));
            hand.line_to((icon_cx + 0.6, icon_cy - 1.5));
            hand.line_to((icon_cx + 0.6, icon_cy - 4.0));
            hand.line_to((icon_cx + 2.3, icon_cy - 4.0));
            hand.line_to((icon_cx + 2.3, icon_cy - 0.5));
            hand.line_to((icon_cx + 3.8, icon_cy - 0.5));
            hand.line_to((icon_cx + 3.8, icon_cy + 2.0));
            hand.line_to((icon_cx + 4.5, icon_cy + 2.0));
            hand.line_to((icon_cx + 4.5, icon_cy + 6.0));
            hand.close_path();
            scene.push(Node::path(icon_id, hand, line_style(options, 1.0)));
        }
        TaskType::Send => {
            // Filled envelope with white flap.
            let w = 11.0;
            let h = 8.0;
            let x = icon_cx - w / 2.0;
            let y = icon_cy - h / 2.0;
            scene.push(Node::rect(
                format!("{icon_id}-body"),
                x,
                y,
                w,
                h,
                filled_style(options),
            ));
            let mut flap = BezPath::new();
            flap.move_to((x, y));
            flap.line_to((icon_cx, y + h * 0.6));
            flap.line_to((x + w, y));
            scene.push(Node::path(icon_id, flap, line_style(options, 1.2)));
        }
        TaskType::Receive => {
            let w = 11.0;
            let h = 8.0;
            let x = icon_cx - w / 2.0;
            let y = icon_cy - h / 2.0;
            scene.push(Node::rect(
                format!("{icon_id}-body"),
                x,
                y,
                w,
                h,
                line_style(options, 1.2),
            ));
            let mut flap = BezPath::new();
            flap.move_to((x, y));
            flap.line_to((icon_cx, y + h * 0.6));
            flap.line_to((x + w, y));
            scene.push(Node::path(icon_id, flap, line_style(options, 1.2)));
        }
        TaskType::BusinessRule => {
            // Table with header and column split.
            let w = 13.0;
            let h = 11.0;
            let x = icon_cx - w / 2.0;
            let y = icon_cy - h / 2.0;
            scene.push(Node::rect(
                format!("{icon_id}-table"),
                x,
                y,
                w,
                h,
                line_style(options, 1.2),
            ));
            let mut rules = BezPath::new();
            rules.move_to((x, y + h * 0.4));
            rules.line_to((x + w, y + h * 0.4));
            rules.move_to((x + w * 0.45, y + h * 0.4));
            rules.line_to((x + w * 0.45, y + h));
            scene.push(Node::path(icon_id, rules, line_style(options, 1.0)));
        }
    }
}

fn draw_gateway(
    scene: &mut Scene,
    id: &str,
    shape: &Shape,
    options: &RenderOptions,
    gateway_type: GatewayType,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((cx, cy)) = shape.bounds.center() else {
        return;
    };
    let half_w = shape.bounds.width / 2.0;
    let half_h = shape.bounds.height / 2.0;
    let diamond = path::diamond(cx, cy, half_w, half_h);
    match gateway_type {
        GatewayType::Exclusive | GatewayType::Parallel | GatewayType::Inclusive => {
            scene.push(Node::path(id, diamond, shape_style(options, 1.5)));
        }
        GatewayType::EventBased | GatewayType::Complex => {
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::UnsupportedSymbol,
                    format!(
                        "{} has no dedicated marker in this prototype; drawing plain diamond",
                        match gateway_type {
                            GatewayType::EventBased => "eventBasedGateway",
                            _ => "complexGateway",
                        }
                    ),
                )
                .with_element_id(shape.bpmn_element.clone()),
            );
            let mut style = shape_style(options, 1.5);
            style.dash_array = Some(vec![4.0, 2.0]);
            scene.push(Node::path(id, diamond, style));
        }
    }
    let inner = half_w.min(half_h) * 0.45;
    match gateway_type {
        GatewayType::Exclusive => {
            let mut cross = BezPath::new();
            cross.move_to((cx - inner, cy - inner));
            cross.line_to((cx + inner, cy + inner));
            cross.move_to((cx + inner, cy - inner));
            cross.line_to((cx - inner, cy + inner));
            scene.push(Node::path(
                format!("{id}-marker"),
                cross,
                line_style(options, 1.6),
            ));
        }
        GatewayType::Parallel => {
            let mut plus = BezPath::new();
            plus.move_to((cx - inner, cy));
            plus.line_to((cx + inner, cy));
            plus.move_to((cx, cy - inner));
            plus.line_to((cx, cy + inner));
            scene.push(Node::path(
                format!("{id}-marker"),
                plus,
                line_style(options, 1.8),
            ));
        }
        GatewayType::Inclusive => {
            scene.push(Node::circle(
                format!("{id}-marker"),
                cx,
                cy,
                inner * 0.8,
                line_style(options, 1.6),
            ));
        }
        _ => {}
    }
}

fn draw_subprocess(scene: &mut Scene, id: &str, shape: &Shape, options: &RenderOptions) {
    let collapsed = shape.is_expanded == Some(false);
    let stroke_width = if collapsed { 1.5 } else { 2.0 };
    scene.push(Node::rounded_rect(
        id,
        shape.bounds.x,
        shape.bounds.y,
        shape.bounds.width,
        shape.bounds.height,
        8.0,
        shape_style(options, stroke_width),
    ));
    // +/[-] marker box at bottom center.
    let box_w = 18.0;
    let box_h = 10.0;
    let Some((cx, _)) = shape.bounds.center() else {
        return;
    };
    let bx = cx - box_w / 2.0;
    let by = shape.bounds.y + shape.bounds.height - box_h - 4.0;
    scene.push(Node::rect(
        format!("{id}-marker-box"),
        bx,
        by,
        box_w,
        box_h,
        shape_style(options, 1.2),
    ));
    let mut plus = BezPath::new();
    let pcx = bx + box_w / 2.0;
    let pcy = by + box_h / 2.0;
    plus.move_to((pcx - 4.0, pcy));
    plus.line_to((pcx + 4.0, pcy));
    if collapsed {
        plus.move_to((pcx, pcy - 3.0));
        plus.line_to((pcx, pcy + 3.0));
    }
    scene.push(Node::path(
        format!("{id}-marker"),
        plus,
        line_style(options, 1.2),
    ));
}

/// Draws a text annotation: open bracket plus its text (text emitted by the
/// label pass).
pub fn draw_annotation(
    scene: &mut Scene,
    annotation: &TextAnnotation,
    shape: &Shape,
    options: &RenderOptions,
) {
    let bracket =
        path::annotation_bracket(shape.bounds.x, shape.bounds.y, shape.bounds.height, 8.0);
    scene.push(Node::path(
        format!("bpmn-shape-{}", annotation.id),
        bracket,
        line_style(options, 1.5),
    ));
}

/// Placeholder for shapes whose semantic element is unknown.
pub fn draw_placeholder(scene: &mut Scene, shape: &Shape, options: &RenderOptions) {
    let mut style = shape_style(options, 1.2);
    style.fill = Some(Paint::Color(options.theme.fallback_fill));
    style.stroke = Some(Paint::Color(options.theme.fallback_stroke));
    style.dash_array = Some(vec![4.0, 2.0]);
    scene.push(Node::rounded_rect(
        format!("bpmn-shape-{}-placeholder", shape.bpmn_element),
        shape.bounds.x,
        shape.bounds.y,
        shape.bounds.width,
        shape.bounds.height,
        4.0,
        style,
    ));
}

/// Conspicuous fallback glyph for modeled-but-unsupported symbols.
pub fn draw_fallback(scene: &mut Scene, id: &str, shape: &Shape, options: &RenderOptions) {
    let mut style = shape_style(options, 1.5);
    style.fill = Some(Paint::Color(options.theme.fallback_fill));
    style.stroke = Some(Paint::Color(options.theme.fallback_stroke));
    scene.push(Node::rounded_rect(
        id,
        shape.bounds.x,
        shape.bounds.y,
        shape.bounds.width,
        shape.bounds.height,
        6.0,
        style,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_model::di::Bounds;
    use bpmn_scene::style::Viewport;

    fn shape() -> Shape {
        Shape {
            id: Some("s1".into()),
            bpmn_element: "Task_1".into(),
            bounds: Bounds::new(0.0, 0.0, 100.0, 80.0),
            label: None,
            is_horizontal: None,
            is_expanded: None,
        }
    }

    #[test]
    fn task_glyph_has_body_and_icon() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 120.0, 100.0));
        draw_task(
            &mut scene,
            "bpmn-shape-Task_1",
            &shape(),
            &RenderOptions::default(),
            TaskType::Service,
        );
        assert!(scene.nodes.len() >= 3);
    }

    #[test]
    fn gateway_markers_differ() {
        let mut exclusive = Scene::new(Viewport::new(0.0, 0.0, 120.0, 100.0));
        let mut parallel = Scene::new(Viewport::new(0.0, 0.0, 120.0, 100.0));
        let mut warnings = Vec::new();
        draw_gateway(
            &mut exclusive,
            "g",
            &shape(),
            &RenderOptions::default(),
            GatewayType::Exclusive,
            &mut warnings,
        );
        draw_gateway(
            &mut parallel,
            "g",
            &shape(),
            &RenderOptions::default(),
            GatewayType::Parallel,
            &mut warnings,
        );
        assert_eq!(exclusive.nodes.len(), parallel.nodes.len());
        assert!(warnings.is_empty());
    }

    #[test]
    fn event_based_gateway_warns() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 120.0, 100.0));
        let mut warnings = Vec::new();
        draw_gateway(
            &mut scene,
            "g",
            &shape(),
            &RenderOptions::default(),
            GatewayType::EventBased,
            &mut warnings,
        );
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code.as_str(), "symbol.unsupported");
    }
}
