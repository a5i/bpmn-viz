//! Argument parsing and command dispatch (SPEC-001 section 8 CLI contract).

mod cli;
mod commands;

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// BPMN 2.0 renderer: render, inspect and validate BPMN diagrams.
#[derive(Parser, Debug)]
#[command(
    name = "bpmn",
    version,
    about = "Render and inspect BPMN 2.0 diagrams without Node.js or a browser",
    after_help = "Exit codes: 0 success, 2 invalid usage, 3 invalid BPMN/DI, 4 IO failure, 5 render failure."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Report style for inspect/validate output.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormatKind {
    /// Human-readable text.
    Text,
    /// One versioned JSON object.
    Json,
}

/// Raster or vector output for render.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderFormat {
    /// SVG output (default when omitted).
    Svg,
    /// PNG output via resvg.
    Png,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Render one diagram of a BPMN file to SVG or PNG.
    Render(RenderCommand),
    /// Summarize a BPMN file's contents.
    Inspect(InspectCommand),
    /// Validate references and geometry of a BPMN file.
    Validate(ValidateCommand),
}

#[derive(Args, Debug)]
pub struct RenderCommand {
    /// Path to the `.bpmn` file.
    pub input: PathBuf,
    /// Output file path or `-` for stdout (SVG only).
    #[arg(short, long)]
    pub output: PathBuf,
    /// Output format; inferred from the output extension when omitted.
    #[arg(short, long, value_enum)]
    pub format: Option<RenderFormat>,
    /// Select a diagram by id, or by zero-based index when the value is a
    /// non-negative number. Defaults to the first diagram (with a warning
    /// when the file contains several).
    #[arg(long)]
    pub diagram: Option<String>,
    /// Positive finite scale for rasterized output only.
    #[arg(long)]
    pub scale: Option<f64>,
}

#[derive(Args, Debug)]
pub struct InspectCommand {
    /// Path to the `.bpmn` file.
    pub input: PathBuf,
    /// Report format.
    #[arg(long, value_enum, default_value_t = OutputFormatKind::Text)]
    pub format: OutputFormatKind,
}

#[derive(Args, Debug)]
pub struct ValidateCommand {
    /// Path to the `.bpmn` file.
    pub input: PathBuf,
    /// Report format.
    #[arg(long, value_enum, default_value_t = OutputFormatKind::Text)]
    pub format: OutputFormatKind,
    /// Treat warnings as errors (exit 3).
    #[arg(long)]
    pub strict: bool,
}

fn main() {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Render(command) => commands::render::run(command),
        Command::Inspect(command) => commands::inspect::run(command, command.format),
        Command::Validate(command) => commands::validate::run(command, command.format),
    };
    let exit = match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {}", error.summary());
            if let cli::CliError::InvalidBpmn(diagnostics) = &error {
                cli::print_diagnostics_text(diagnostics);
            }
            error.exit_code()
        }
    };
    std::process::exit(exit as i32);
}
