mod aabb;
mod app;
mod camera;
mod cli;
mod clip;
mod diff;
mod mcp;
mod mcp_docs;
mod mesh;
mod probe;
mod render;
mod shade;
mod spatial;
mod topology;

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;

use crate::aabb::AabbBox;
use crate::app::App;
use crate::camera::{Camera, CameraArgs};
use crate::cli::{Cli, Commands};
use crate::diff::{mesh_diff, volume_in_box};
use crate::mesh::Mesh;
use crate::probe::{is_inside, parse_axis, slice_mesh};
use crate::render::{
    render_deviation, render_framed, render_overlay, resolve_size,
};
use glam::Vec3;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Mcp) => {
            let rt = tokio::runtime::Runtime::new().context("tokio runtime")?;
            return rt.block_on(mcp::run_mcp());
        }
        Some(Commands::Diff {
            a,
            b,
            clip,
            samples,
            grid,
        }) => {
            let clip = parse_clip(clip)?;
            let ma = load_maybe_clip(&a, clip)?;
            let mb = load_maybe_clip(&b, clip)?;
            let result = mesh_diff(&ma, &mb, samples, grid, clip);
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        Some(Commands::VolumeInBox {
            file,
            box_vals,
            grid,
        }) => {
            let b = AabbBox::from_slice(&box_vals).map_err(|e| anyhow::anyhow!(e))?;
            let mesh = Mesh::load(&file)?;
            let result = volume_in_box(&mesh, b, grid);
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        Some(Commands::Stats { file, file_b, clip }) => {
            let clip = parse_clip(clip)?;
            let a = load_maybe_clip(&file, clip)?;
            if let Some(bpath) = file_b {
                let b = load_maybe_clip(&bpath, clip)?;
                let mut obj = serde_json::json!({ "a": a.stats, "b": b.stats });
                if let Some(c) = clip {
                    obj["clip"] = serde_json::json!([c.x, c.y, c.z, c.dx, c.dy, c.dz]);
                }
                println!("{}", serde_json::to_string_pretty(&obj)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&a.stats)?);
            }
            return Ok(());
        }
        Some(Commands::Overlay {
            a,
            b,
            clip,
            width,
            height,
            preset,
            yaw,
            pitch,
            zoom,
        }) => {
            let clip = parse_clip(clip)?;
            let ma = load_maybe_clip(&a, clip)?;
            let mb = load_maybe_clip(&b, clip)?;
            let camera = Camera::from_mesh_and_args(
                &ma,
                &CameraArgs {
                    preset,
                    yaw,
                    pitch,
                    zoom,
                    pan_x: None,
                    pan_y: None,
                    focus: None,
                    radius: None,
                    clip,
                },
            );
            let (w, h) = resolve_size(&ma, width, height);
            print!("{}", render_overlay(&ma, &mb, &camera, w, h, clip));
            return Ok(());
        }
        Some(Commands::Deviation {
            file,
            reference,
            clip,
            width,
            height,
            preset,
            yaw,
            pitch,
            zoom,
        }) => {
            let clip = parse_clip(clip)?;
            let mesh = load_maybe_clip(&file, clip)?;
            let href = load_maybe_clip(&reference, clip)?;
            let camera = Camera::from_mesh_and_args(
                &mesh,
                &CameraArgs {
                    preset,
                    yaw,
                    pitch,
                    zoom,
                    pan_x: None,
                    pan_y: None,
                    focus: None,
                    radius: None,
                    clip,
                },
            );
            let (w, h) = resolve_size(&mesh, width, height);
            print!("{}", render_deviation(&mesh, &href, &camera, w, h, clip));
            return Ok(());
        }
        Some(Commands::IsInside { file, point, clip }) => {
            if point.len() != 3 {
                bail!("--point needs x,y,z");
            }
            let clip = parse_clip(clip)?;
            let mesh = load_maybe_clip(&file, clip)?;
            let result = is_inside(&mesh, Vec3::new(point[0], point[1], point[2]));
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        Some(Commands::Slice {
            file,
            axis,
            value,
            clip,
        }) => {
            let axis = parse_axis(&axis).map_err(|e| anyhow::anyhow!(e))?;
            let clip = parse_clip(clip)?;
            let mesh = load_maybe_clip(&file, clip)?;
            let result = slice_mesh(&mesh, axis, value);
            println!("{}", serde_json::to_string_pretty(&result)?);
            print!("{}", result.ascii_preview);
            return Ok(());
        }
        None => {}
    }

    let file = cli
        .file
        .clone()
        .context("FILE is required (or use a subcommand). See --help.")?;
    run_view(file, &cli)
}

fn parse_clip(clip: Option<Vec<f32>>) -> Result<Option<AabbBox>> {
    match clip {
        None => Ok(None),
        Some(v) => Ok(Some(AabbBox::from_slice(&v).map_err(|e| anyhow::anyhow!(e))?)),
    }
}

fn load_maybe_clip(path: &PathBuf, clip: Option<AabbBox>) -> Result<Mesh> {
    let mesh = Mesh::load(path)?;
    Ok(match clip {
        Some(c) => mesh.clipped(c),
        None => mesh,
    })
}

fn run_view(file: PathBuf, cli: &Cli) -> Result<()> {
    if !file.exists() {
        bail!("STL not found: {}", file.display());
    }

    let clip = parse_clip(cli.clip.clone())?;
    let mesh = load_maybe_clip(&file, clip)?;
    let focus = match &cli.focus {
        None => None,
        Some(v) if v.len() == 3 => Some([v[0], v[1], v[2]]),
        Some(_) => bail!("--focus needs x,y,z"),
    };
    let camera = Camera::from_mesh_and_args(
        &mesh,
        &CameraArgs {
            preset: cli.preset,
            yaw: cli.yaw,
            pitch: cli.pitch,
            zoom: cli.zoom,
            pan_x: cli.pan_x,
            pan_y: cli.pan_y,
            focus,
            radius: cli.radius,
            clip,
        },
    );

    if cli.snapshot {
        let (w, h) = match (cli.width, cli.height) {
            (None, None) => {
                snapshot_size_terminal().unwrap_or_else(|| resolve_size(&mesh, None, None))
            }
            (w, h) => resolve_size(&mesh, w, h),
        };
        let ascii = render_framed(&mesh, &camera, cli.shade, w, h, clip);
        let mut out = io::stdout();
        out.write_all(ascii.as_bytes())?;
        out.flush()?;
        return Ok(());
    }

    if !io::stdout().is_terminal() {
        bail!("interactive mode needs a terminal; use --snapshot");
    }

    App::new(mesh, camera, cli.shade).run()
}

fn snapshot_size_terminal() -> Option<(usize, usize)> {
    let (cols, rows) = crossterm::terminal::size().ok()?;
    let w = (cols as usize).saturating_sub(2).clamp(40, 200);
    let h = (rows as usize).saturating_sub(4).clamp(12, 80);
    Some((w, h))
}
