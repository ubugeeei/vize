//! # vize
//!
//! Vize - High-performance Vue.js toolchain in Rust.
//!
//! ## Name Origin
//!
//! **Vize** (/viːz/) is named after Vizier + Visor + Advisor — a wise tool
//! that sees through your code. This crate is the gateway to all Vize functionality,
//! providing a unified command-line interface for compiling Vue SFC files
//! with native performance.

mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vize")]
#[command(about = "High-performance Vue.js toolchain in Rust", long_about = None)]
#[command(version, disable_version_flag = true)]
struct Cli {
    /// Print version
    #[arg(short = 'v', short_alias = 'V', long, action = clap::ArgAction::Version)]
    version: (),
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile Vue SFC files (default command)
    #[command(visible_alias = "atelier")]
    Build(commands::build::BuildArgs),

    /// Format Vue SFC files
    #[command(visible_alias = "glyph")]
    Fmt(commands::fmt::FmtArgs),

    /// Lint Vue SFC files
    #[command(visible_alias = "patina")]
    Lint(commands::lint::LintArgs),

    /// Type check Vue SFC files
    Check(commands::check::CheckArgs),

    /// Start type check JSON-RPC server (Unix only)
    #[cfg(unix)]
    CheckServer(commands::check_server::CheckServerArgs),

    /// Start component gallery server
    Musea(commands::musea::MuseaArgs),

    /// Start Language Server Protocol server
    #[command(visible_alias = "maestro")]
    Lsp(commands::lsp::LspArgs),

    /// IDE integration - LSP server and editor extension management
    Ide(commands::ide::IdeArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build(args)) => commands::build::run(args),
        Some(Commands::Fmt(args)) => commands::fmt::run(args),
        Some(Commands::Lint(args)) => commands::lint::run(args),
        Some(Commands::Check(args)) => commands::check::run(args),
        #[cfg(unix)]
        Some(Commands::CheckServer(args)) => commands::check_server::run(args),
        Some(Commands::Musea(args)) => commands::musea::run(args),
        Some(Commands::Lsp(args)) => commands::lsp::run(args),
        Some(Commands::Ide(args)) => commands::ide::run(args),
        None => {
            // Default to build command with default args
            commands::build::run(commands::build::BuildArgs::default());
        }
    }
}
