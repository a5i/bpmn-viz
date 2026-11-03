//! Scene → SVG string writer.

use bpmn_scene::style::{Marker, Paint, Style};
use bpmn_scene::{Node, Scene};
use kurbo::{BezPath, PathEl};
use std::fmt::Write;

use crate::{escape_attribute, escape_text, format_number};

/// Serializes `scene` into a deterministic SVG document.
pub fn to_svg(scene: &Scene) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">",
        format_number(scene.viewport.width),
        format_number(scene.viewport.height),
        format_number(scene.viewport.x),
        format_number(scene.viewport.y),
        format_number(scene.viewport.width),
        format_number(scene.viewport.height),
    );
    if !scene.defs.is_empty() {
        out.push_str("<defs>\n");
        for marker in &scene.defs {
            write_marker(&mut out, marker);
        }
        out.push_str("</defs>\n");
    }
    // White background rect for consistent raster output.
    let _ = writeln!(
        out,
        "<rect id=\"bpmn-background\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>",
        format_number(scene.viewport.x),
        format_number(scene.viewport.y),
        format_number(scene.viewport.width),
        format_number(scene.viewport.height),
    );
    for node in &scene.nodes {
        write_node(&mut out, node, 0);
    }
    out.push_str("</svg>\n");
    out
}

fn write_marker(out: &mut String, marker: &Marker) {
    let _ = write!(
        out,
        "<marker id=\"{}\" viewBox=\"0 0 {} {}\" refX=\"{}\" refY=\"{}\" markerWidth=\"{}\" markerHeight=\"{}\" orient=\"{}\">",
        escape_attribute(&marker.id),
        format_number(marker.view_box_size),
        format_number(marker.view_box_size),
        format_number(marker.ref_x),
        format_number(marker.ref_y),
        format_number(marker.marker_width),
        format_number(marker.marker_height),
        marker.orient.css(),
    );
    let _ = writeln!(
        out,
        "<path d=\"{}\" fill=\"{}\"/></marker>",
        path_data(&marker.data),
        marker.fill.css(),
    );
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_node(out: &mut String, node: &Node, depth: usize) {
    indent(out, depth);
    match node {
        Node::Group {
            id,
            transform,
            children,
        } => {
            let mut attributes = String::new();
            if let Some(id) = id {
                attributes.push_str(&format!(" id=\"{}\"", escape_attribute(id)));
            }
            if let Some(transform) = transform {
                attributes.push_str(&format!(
                    " transform=\"{}\"",
                    transform_attribute(*transform)
                ));
            }
            if children.is_empty() {
                let _ = writeln!(out, "<g{attributes}/>");
                return;
            }
            let _ = writeln!(out, "<g{attributes}>");
            for child in children {
                write_node(out, child, depth + 1);
            }
            indent(out, depth);
            let _ = writeln!(out, "</g>");
        }
        Node::Path { id, data, style } => {
            let _ = write!(
                out,
                "<path{} d=\"{}\"{}",
                id_attribute(id.as_deref()),
                path_data(data),
                style_attributes(style),
            );
            let _ = writeln!(out, "/>");
        }
        Node::Rect {
            id,
            x,
            y,
            width,
            height,
            corner_radius,
            style,
        } => {
            let radius = if *corner_radius > 0.0 {
                format!(
                    " rx=\"{}\" ry=\"{}\"",
                    format_number(*corner_radius),
                    format_number(*corner_radius)
                )
            } else {
                String::new()
            };
            let _ = write!(
                out,
                "<rect{} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}{}",
                id_attribute(id.as_deref()),
                format_number(*x),
                format_number(*y),
                format_number(*width),
                format_number(*height),
                radius,
                style_attributes(style),
            );
            let _ = writeln!(out, "/>");
        }
        Node::Ellipse {
            id,
            cx,
            cy,
            rx,
            ry,
            style,
        } => {
            let _ = write!(
                out,
                "<ellipse{} cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{}",
                id_attribute(id.as_deref()),
                format_number(*cx),
                format_number(*cy),
                format_number(*rx),
                format_number(*ry),
                style_attributes(style),
            );
            let _ = writeln!(out, "/>");
        }
        Node::Text {
            id,
            x,
            y,
            content,
            style,
            anchor,
        } => {
            let _ = write!(
                out,
                "<text{} x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" text-anchor=\"{}\" fill=\"{}\"",
                id_attribute(id.as_deref()),
                format_number(*x),
                format_number(*y),
                escape_attribute(&style.family),
                format_number(style.size),
                style.weight.css(),
                anchor.css(),
                style.fill.css(),
            );
            if style.italic {
                out.push_str(" font-style=\"italic\"");
            }
            let _ = writeln!(out, ">{}</text>", escape_text(content));
        }
    }
}

fn id_attribute(id: Option<&str>) -> String {
    match id {
        Some(id) => format!(" id=\"{}\"", escape_attribute(id)),
        None => String::new(),
    }
}

fn style_attributes(style: &Style) -> String {
    let mut attributes = String::new();
    match style.fill {
        Some(Paint::None) => attributes.push_str(" fill=\"none\""),
        Some(Paint::Color(color)) => {
            let _ = write!(attributes, " fill=\"{}\"", color.css());
        }
        None => {}
    }
    match style.stroke {
        Some(Paint::None) => attributes.push_str(" stroke=\"none\""),
        Some(Paint::Color(color)) => {
            let _ = write!(
                attributes,
                " stroke=\"{}\" stroke-width=\"{}\"",
                color.css(),
                format_number(style.stroke_width)
            );
        }
        None => {}
    }
    if let Some(dash_array) = &style.dash_array {
        let dashes: Vec<String> = dash_array.iter().map(|v| format_number(*v)).collect();
        let _ = write!(attributes, " stroke-dasharray=\"{}\"", dashes.join(" "));
    }
    if let Some(marker) = &style.marker_end {
        let _ = write!(
            attributes,
            " marker-end=\"url(#{})\"",
            escape_attribute(marker)
        );
    }
    if let Some(marker) = &style.marker_start {
        let _ = write!(
            attributes,
            " marker-start=\"url(#{})\"",
            escape_attribute(marker)
        );
    }
    attributes
}

fn transform_attribute(transform: kurbo::Affine) -> String {
    // kurbo coeffs are row-major [a, b, c, d, e, f]; SVG matrix(a,b,c,d,e,f).
    let [a, b, c, d, e, f] = transform.as_coeffs();
    format!(
        "matrix({},{},{},{},{},{})",
        format_number(a),
        format_number(b),
        format_number(c),
        format_number(d),
        format_number(e),
        format_number(f)
    )
}

fn path_data(path: &BezPath) -> String {
    let mut out = String::new();
    for element in path.elements() {
        match element {
            PathEl::MoveTo(point) => {
                let _ = write!(
                    out,
                    "M {} {}",
                    format_number(point.x),
                    format_number(point.y)
                );
            }
            PathEl::LineTo(point) => {
                let _ = write!(
                    out,
                    " L {} {}",
                    format_number(point.x),
                    format_number(point.y)
                );
            }
            PathEl::QuadTo(control, point) => {
                let _ = write!(
                    out,
                    " Q {} {} {} {}",
                    format_number(control.x),
                    format_number(control.y),
                    format_number(point.x),
                    format_number(point.y)
                );
            }
            PathEl::CurveTo(c1, c2, point) => {
                let _ = write!(
                    out,
                    " C {} {} {} {} {} {}",
                    format_number(c1.x),
                    format_number(c1.y),
                    format_number(c2.x),
                    format_number(c2.y),
                    format_number(point.x),
                    format_number(point.y)
                );
            }
            PathEl::ClosePath => out.push_str(" Z"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_scene::style::{Rgba, TextStyle, Viewport};
    use bpmn_scene::text::TextAnchor;

    #[test]
    fn deterministic_output() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 100.0, 50.0));
        let style = Style {
            fill: Some(Paint::Color(Rgba::WHITE)),
            stroke: Some(Paint::Color(Rgba::BLACK)),
            stroke_width: 1.5,
            dash_array: None,
            marker_end: None,
            marker_start: None,
        };
        scene.push(Node::rect("a", 1.0, 2.0, 10.0, 20.0, style.clone()));
        scene.push(Node::text(
            "t",
            5.0,
            5.0,
            "a & b",
            TextStyle::default(),
            TextAnchor::Middle,
        ));
        let first = to_svg(&scene);
        let second = to_svg(&scene);
        assert_eq!(first, second);
        assert!(first.contains("fill=\"#ffffff\""));
        assert!(first.contains("a &amp; b"));
        assert!(first.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn groups_and_transforms_serialize() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 100.0, 50.0));
        scene.wrap_next(Some("g".into()), kurbo::Affine::translate((10.0, 20.0)));
        scene.push(Node::rect("r", 0.0, 0.0, 5.0, 5.0, Style::default()));
        let svg = to_svg(&scene);
        assert!(svg.contains("<g id=\"g\" transform=\"matrix(1,0,0,1,10,20)\""));
        assert!(svg.contains("</g>"));
    }

    #[test]
    fn path_data_formats_elements() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((10.25, 0.0));
        path.close_path();
        let data = path_data(&path);
        assert_eq!(data, "M 0 0 L 10.25 0 Z");
    }
}
