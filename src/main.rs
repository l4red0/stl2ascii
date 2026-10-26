mod app;
mod camera;
mod cli;
mod mcp;
mod mesh;
mod render;
mod shade;

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;

use crate::app::App;
use crate::camera::Camera;
use crate::cli::{Cli, Commands};
use crate::mesh::Mesh;
use crate::render::{default_snapshot_size, render_framed};

fn main() -> Result<()> {
    let cli = Cli::parse();

    if matches!(cli.command, Some(Commands::Mcp)) {
        let rt = tokio::runtime::Runtime::new().context("tokio runtime")?;
        return rt.block_on(mcp::run_mcp());
    }

    let file = cli
        .file
        .clone()
        .context("FILE is required (or use `stl2ascii mcp`). See --help.")?;

    run_view(file, &cli)
}

fn run_view(file: PathBuf, cli: &Cli) -> Result<()> {
    if !file.exists() {
        bail!("STL not found: {}", file.display());
    }

    let mesh = Mesh::load(&file)?;
    let camera = Camera::from_mesh_and_args(
        &mesh,
        cli.preset,
        cli.yaw,
        cli.pitch,
        cli.zoom,
        cli.pan_x,
        cli.pan_y,
    );

    if cli.snapshot {
        let (w, h) = snapshot_size();
        let ascii = render_framed(&mesh, &camera, cli.shade, w, h);
        let mut out = io::stdout();
        out.write_all(ascii.as_bytes())?;
        out.flush()?;
        return Ok(());
    }

    if !io::stdout().is_terminal() {
        bail!("interactive mode needs a terminal; use --snapshot for piped stdout");
    }

    App::new(mesh, camera, cli.shade).run()
}

fn snapshot_size() -> (usize, usize) {
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        let w = (cols as usize).saturating_sub(2).clamp(40, 200);
        let h = (rows as usize).saturating_sub(4).clamp(12, 80);
        (w, h)
    } else {
        default_snapshot_size()
    }
}
