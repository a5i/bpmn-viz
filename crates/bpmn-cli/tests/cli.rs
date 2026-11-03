//! Integration tests: CLI behavior, exit codes and output contracts (AC-08,
//! AC-09, AC-10). Fixtures live in `tests/fixtures/` at the workspace root.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn bpmn(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_bpmn"))
        .args(args)
        .output()
        .expect("spawn bpmn");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

const MINIMAL: &str = "minimal/start-task-end.bpmn";

#[test]
fn render_svg_success_exit_zero() {
    let input = fixture(MINIMAL);
    let output = std::env::temp_dir().join(format!("bpmn-it-svg-{}.svg", std::process::id()));
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let svg = std::fs::read_to_string(&output).expect("output exists");
    assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(svg.contains("id=\"bpmn-shape-Task_1\""));
    std::fs::remove_file(&output).unwrap();
}

#[test]
fn render_stdout_dash_writes_svg() {
    let input = fixture(MINIMAL);
    let (code, stdout, _stderr) = bpmn(&["render", input.to_str().unwrap(), "--output", "-"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""));
}

#[test]
fn render_png_with_scale_exit_zero() {
    let input = fixture(MINIMAL);
    let output = std::env::temp_dir().join(format!("bpmn-it-png-{}.png", std::process::id()));
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--format",
        "png",
        "--scale",
        "2",
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let png = std::fs::read(&output).expect("png exists");
    assert_eq!(bpmn_raster::png_dimensions(&png), Some((632, 200)));
    std::fs::remove_file(&output).unwrap();
}

#[test]
fn format_extension_mismatch_is_usage_error() {
    let input = fixture(MINIMAL);
    let output = std::env::temp_dir().join("will-not-be-written.svg");
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--format",
        "png",
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(!output.exists());
}

#[test]
fn scale_with_svg_is_usage_error() {
    let input = fixture(MINIMAL);
    let (code, _stdout, _stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--scale",
        "2",
        "--output",
        "-",
    ]);
    assert_eq!(code, 2);
}

#[test]
fn duplicate_id_exits_three_with_diagnostic() {
    let input = fixture("invalid/duplicate-id.bpmn");
    let (code, stdout, _stderr) = bpmn(&["validate", input.to_str().unwrap()]);
    assert_eq!(code, 3, "stdout: {stdout}");
    assert!(
        stdout.contains("reference.duplicate_id"),
        "stdout: {stdout}"
    );
}

#[test]
fn validate_json_has_stable_codes() {
    let input = fixture("invalid/dangling-reference.bpmn");
    let (code, stdout, _stderr) = bpmn(&["validate", "--format", "json", input.to_str().unwrap()]);
    assert_eq!(code, 3);
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(value["contract_version"], 1);
    assert_eq!(value["command"], "validate");
    assert!(value["error_count"].as_u64().unwrap() >= 2);
    let codes: Vec<&str> = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"reference.dangling"));
    assert!(codes.contains(&"di.missing_shape"));
}

#[test]
fn missing_diagram_interchange_is_error() {
    let input = fixture("invalid/missing-di.bpmn");
    let (code, stdout, _stderr) = bpmn(&["validate", input.to_str().unwrap()]);
    assert_eq!(code, 3, "stdout: {stdout}");
    assert!(
        stdout.contains("di.missing_interchange"),
        "stdout: {stdout}"
    );
    // Render must refuse too (AC-09: no hidden layout fallback).
    let output = std::env::temp_dir().join(format!("bpmn-it-nodi-{}.svg", std::process::id()));
    let (code, _stdout, _stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 3);
    assert!(!output.exists());
}

#[test]
fn nan_bounds_are_geometry_errors() {
    let input = fixture("invalid/nan-bounds.bpmn");
    let (code, stdout, _stderr) = bpmn(&["validate", input.to_str().unwrap()]);
    assert_eq!(code, 3, "stdout: {stdout}");
    assert!(stdout.contains("geometry.invalid"), "stdout: {stdout}");
}

#[test]
fn multi_diagram_selection_by_index_and_id() {
    let input = fixture("edge-cases/multi-diagram.bpmn");
    let first = std::env::temp_dir().join(format!("bpmn-it-d0-{}.svg", std::process::id()));
    let second = std::env::temp_dir().join(format!("bpmn-it-d1-{}.svg", std::process::id()));

    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--diagram",
        "1",
        "--output",
        second.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let second_svg = std::fs::read_to_string(&second).unwrap();
    assert!(second_svg.contains("x=\"500\""), "second diagram geometry");

    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--diagram",
        "Diagram_First",
        "--output",
        first.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let first_svg = std::fs::read_to_string(&first).unwrap();
    assert!(first_svg.contains("id=\"bpmn-shape-Gateway_P\""));

    // Out-of-range index is a BPMN error (exit 3).
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--diagram",
        "9",
        "--output",
        "-",
    ]);
    assert_eq!(code, 3, "stderr: {stderr}");
    assert!(stderr.contains("diagram.not_found"), "stderr: {stderr}");

    std::fs::remove_file(&first).unwrap();
    std::fs::remove_file(&second).unwrap();
}

#[test]
fn unicode_and_negative_coordinates_render() {
    let input = fixture("edge-cases/unicode.bpmn");
    let output = std::env::temp_dir().join(format!("bpmn-it-uni-{}.svg", std::process::id()));
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let svg = std::fs::read_to_string(&output).unwrap();
    assert!(svg.contains("&amp;"), "ampersand escaped");
    assert!(svg.contains("Prüfung"), "unicode preserved");
    assert!(svg.contains("&lt;Test&gt;"), "angle brackets escaped");
    assert!(svg.contains("完了 🎉"));
    assert!(
        svg.contains("viewBox=\"-270.5 -72"),
        "negative origin preserved"
    );
    std::fs::remove_file(&output).unwrap();
}

#[test]
fn malformed_and_xxe_inputs_do_not_crash() {
    let broken = fixture("malformed/broken-xml.bpmn");
    let (code, _stdout, stderr) = bpmn(&["validate", broken.to_str().unwrap()]);
    assert_eq!(code, 3, "stderr: {stderr}");
    assert!(stderr.contains("xml.syntax") || stderr.contains("definitions.missing_root"));

    let _xxe = fixture("malformed/xxe.bpmn");
    let xxe = fixture("malformed/xxe.bpmn");
    let (code, _stdout, _stderr) = bpmn(&["validate", xxe.to_str().unwrap()]);
    // Entities are never expanded: either a hard parse error or a tolerated
    // warning with the attribute ignored. No leak of file content either way.
    assert!(code == 0 || code == 3, "unexpected exit {code}");
    let output = std::env::temp_dir().join(format!("bpmn-it-xxe-{}.svg", std::process::id()));
    let (code, stdout, _stderr) = bpmn(&[
        "render",
        xxe.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    if code == 0 {
        assert!(!stdout.contains("/etc/passwd"));
        assert!(!stdout.contains("root:"));
    }
    if output.exists() {
        let svg = std::fs::read_to_string(&output).unwrap();
        assert!(
            !svg.contains("/etc/passwd"),
            "entity expansion must never happen"
        );
        assert!(!svg.contains("root:"), "entity expansion must never happen");
        std::fs::remove_file(&output).unwrap();
    }
}

#[test]
fn inspect_reports_structure_in_text_and_json() {
    let input = fixture("edge-cases/collaboration-pool-lanes.bpmn");
    let (code, stdout, _stderr) = bpmn(&["inspect", input.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("participants: 2"));
    assert!(stdout.contains("lanes:       2"));

    let (code, stdout, _stderr) = bpmn(&["inspect", "--format", "json", input.to_str().unwrap()]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(value["contract_version"], 1);
    assert_eq!(value["diagram_count"], 1);
    assert_eq!(value["participants"], 2);
    assert_eq!(value["message_flows"], 1);
}

#[test]
fn collaboration_and_subprocess_render() {
    let collaboration = fixture("edge-cases/collaboration-pool-lanes.bpmn");
    let output = std::env::temp_dir().join(format!("bpmn-it-collab-{}.svg", std::process::id()));
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        collaboration.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let svg = std::fs::read_to_string(&output).unwrap();
    assert!(svg.contains("id=\"bpmn-shape-Pool_Buyer\""));
    assert!(svg.contains("id=\"bpmn-shape-Lane_Browsing\""));
    assert!(svg.contains("id=\"bpmn-edge-MessageFlow_1\""));
    std::fs::remove_file(&output).unwrap();

    let subprocess = fixture("edge-cases/expanded-subprocess.bpmn");
    let output = std::env::temp_dir().join(format!("bpmn-it-sub-{}.svg", std::process::id()));
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        subprocess.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let svg = std::fs::read_to_string(&output).unwrap();
    assert!(svg.contains("id=\"bpmn-shape-Sub_Booking\""));
    assert!(svg.contains("id=\"bpmn-shape-Sub_Task_Flight\""));
    std::fs::remove_file(&output).unwrap();
}

#[test]
fn arbitrary_prefixes_parse_and_render() {
    let input = fixture("edge-cases/custom-prefixes.bpmn");
    let (code, stdout, _stderr) = bpmn(&["validate", input.to_str().unwrap()]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let output = std::env::temp_dir().join(format!("bpmn-it-prefix-{}.svg", std::process::id()));
    let (code, _stdout, stderr) = bpmn(&[
        "render",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let svg = std::fs::read_to_string(&output).unwrap();
    // Default-flow tick and conditional diamond present.
    assert!(svg.contains("id=\"bpmn-edge-F3-default\""));
    assert!(svg.contains("marker-start=\"url(#bpmn-conditional)\""));
    std::fs::remove_file(&output).unwrap();
}

#[test]
fn missing_input_is_io_error() {
    let (code, _stdout, _stderr) = bpmn(&["validate", "/nonexistent/file.bpmn"]);
    assert_eq!(code, 4);
}

#[test]
fn strict_promotes_warnings() {
    let input = fixture("edge-cases/custom-prefixes.bpmn");
    let (code, _stdout, _stderr) = bpmn(&["validate", input.to_str().unwrap()]);
    assert_eq!(code, 0);
    let (code, _stdout, _stderr) = bpmn(&["validate", "--strict", input.to_str().unwrap()]);
    assert_eq!(code, 3, "warnings become errors under --strict");
}
