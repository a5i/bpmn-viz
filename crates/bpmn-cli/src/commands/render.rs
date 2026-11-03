//! `bpmn render` implementation (SPEC-001 section 8).
//!
//! Refuses mismatched explicit format vs output extension, keeps output
//! writes atomic, and defaults to the first diagram with an explicit
//! warning when a file contains several.

use std::path::Path;

use bpmn_model::diagnostic::Diagnostic;
use bpmn_model::diagnostic::DiagnosticCode;
use bpmn_raster::{PngOptions, RasterError};
use bpmn_render::{RenderError, RenderOptions};

use crate::cli::{CliError, ExitCode, write_atomic, write_stdout};
use crate::{RenderCommand, RenderFormat};

pub(crate) fn run(command: &RenderCommand) -> Result<ExitCode, CliError> {
    let format = resolve_format(command)?;
    if command.scale.is_some() && format != RenderFormat::Png {
        return Err(CliError::Usage(
            "--scale only applies to rasterized output (--format png)".to_string(),
        ));
    }
    if let Some(scale) = command.scale
        && (!scale.is_finite() || scale <= 0.0)
    {
        return Err(CliError::Usage(format!(
            "--scale must be a positive finite number, got {scale}"
        )));
    }

    let (document, report) = crate::commands::load_and_validate(&command.input)?;
    let mut warnings: Vec<Diagnostic> = document.parse_diagnostics.clone();
    warnings.extend(report.diagnostics.iter().cloned());

    let diagram = select_diagram(&document, command.diagram.as_deref())?;
    if command.diagram.is_none() && document.diagram_count() > 1 {
        eprintln!(
            "warning: file contains {} diagrams; rendering the first (use --diagram to select)",
            document.diagram_count()
        );
    }

    let rendered = bpmn_render::render_diagram(&document, diagram, &RenderOptions::default())
        .map_err(|error| render_error_to_cli(&error))?;
    warnings.extend(rendered.diagnostics.iter().cloned());

    let svg = bpmn_svg::to_svg(&rendered.scene);
    let size = rendered.scene.viewport;
    let bytes = match format {
        RenderFormat::Svg => svg.into_bytes(),
        RenderFormat::Png => {
            let options = PngOptions {
                scale: command.scale.unwrap_or(1.0),
                ..PngOptions::default()
            };
            let png = bpmn_raster::render_png_with_diagnostics(&svg, &options)
                .map_err(|error| raster_error_to_cli(&error))?;
            warnings.extend(png.diagnostics.iter().cloned());
            png.bytes
        }
    };

    write_output(&command.output, format, &bytes)?;

    // Summary to stderr; stdout carries only explicit `-` SVG output.
    let output_description = if command.output == Path::new("-") {
        "-".to_string()
    } else {
        command.output.display().to_string()
    };
    eprintln!(
        "rendered {} -> {} ({}x{}, {} bytes, {} warning(s))",
        command.input.display(),
        output_description,
        crate::commands::inspect::format_number(size.width),
        crate::commands::inspect::format_number(size.height),
        bytes.len(),
        warnings.len(),
    );
    for warning in &warnings {
        eprintln!("warning: {warning}");
    }
    Ok(ExitCode::Success)
}

fn resolve_format(command: &RenderCommand) -> Result<RenderFormat, CliError> {
    if let Some(format) = command.format {
        // Explicit format must agree with the extension when there is one.
        if let Some(extension) = command.output.extension().and_then(|e| e.to_str()) {
            let matches = match format {
                RenderFormat::Svg => extension.eq_ignore_ascii_case("svg"),
                RenderFormat::Png => extension.eq_ignore_ascii_case("png"),
            };
            if !matches && command.output != Path::new("-") {
                return Err(CliError::Usage(format!(
                    "explicit --format {:?} contradicts output extension .{extension}; omit --format or fix the extension",
                    format_str(format),
                )));
            }
        }
        return Ok(format);
    }
    if let Some(extension) = command.output.extension().and_then(|e| e.to_str()) {
        if extension.eq_ignore_ascii_case("png") {
            return Ok(RenderFormat::Png);
        }
        if extension.eq_ignore_ascii_case("svg") {
            return Ok(RenderFormat::Svg);
        }
        return Err(CliError::Usage(format!(
            "cannot infer output format from extension .{extension}; pass --format svg or --format png"
        )));
    }
    Ok(RenderFormat::Svg)
}

fn format_str(format: RenderFormat) -> &'static str {
    match format {
        RenderFormat::Svg => "svg",
        RenderFormat::Png => "png",
    }
}

fn select_diagram<'a>(
    document: &'a bpmn_model::BpmnDocument,
    selector: Option<&str>,
) -> Result<&'a bpmn_model::di::Diagram, CliError> {
    match selector {
        None => document.first_diagram().ok_or_else(|| {
            CliError::InvalidBpmn(vec![Diagnostic::error(
                DiagnosticCode::MissingDiagramInterchange,
                "document contains no diagram to render",
            )])
        }),
        Some(selector) => {
            if let Ok(index) = selector.parse::<usize>() {
                return document.diagram_by_index(index).ok_or_else(|| {
                    CliError::InvalidBpmn(vec![Diagnostic::error(
                        DiagnosticCode::DiagramNotFound,
                        format!(
                            "diagram index {index} out of range (document has {} diagrams)",
                            document.diagram_count()
                        ),
                    )])
                });
            }
            document.diagram_by_id(selector).ok_or_else(|| {
                CliError::InvalidBpmn(vec![Diagnostic::error(
                    DiagnosticCode::DiagramNotFound,
                    format!("no diagram with id `{selector}`"),
                )])
            })
        }
    }
}

fn write_output(output: &Path, format: RenderFormat, bytes: &[u8]) -> Result<(), CliError> {
    if output == Path::new("-") {
        if format == RenderFormat::Png {
            return Err(CliError::Usage(
                "PNG output to stdout is not supported; write to a file".to_string(),
            ));
        }
        return write_stdout(bytes);
    }
    write_atomic(output, bytes)
}

fn render_error_to_cli(error: &RenderError) -> CliError {
    match error {
        RenderError::MissingGeometry { element_id } => CliError::InvalidBpmn(vec![
            Diagnostic::error(
                DiagnosticCode::MissingShape,
                format!("cannot render element `{element_id}`: no diagram interchange available"),
            )
            .with_element_id(element_id.clone()),
        ]),
        RenderError::EmptyDiagram => CliError::InvalidBpmn(vec![Diagnostic::error(
            DiagnosticCode::MissingDiagramInterchange,
            "diagram has no renderable geometry",
        )]),
    }
}

fn raster_error_to_cli(error: &RasterError) -> CliError {
    match error {
        RasterError::TooLarge { .. } | RasterError::Encoding => CliError::Render(error.to_string()),
        RasterError::InvalidSvg { .. } => CliError::Render(error.to_string()),
        RasterError::InvalidScale { .. } => CliError::Usage(error.to_string()),
    }
}
