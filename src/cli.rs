use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CameraPreset {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    Iso,
}

impl CameraPreset {
    pub fn all() -> &'static [CameraPreset] {
        &[
            Self::Front,
            Self::Back,
            Self::Left,
            Self::Right,
            Self::Top,
            Self::Bottom,
            Self::Iso,
        ]
    }

    pub fn yaw_pitch(self) -> (f32, f32) {
        match self {
            Self::Front => (0.0, 0.0),
            Self::Back => (180.0, 0.0),
            Self::Left => (-90.0, 0.0),
            Self::Right => (90.0, 0.0),
            Self::Top => (0.0, 89.0),
            Self::Bottom => (0.0, -89.0),
            Self::Iso => (45.0, 35.264),
        }
    }
}

impl std::fmt::Display for CameraPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Front => "front",
            Self::Back => "back",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Iso => "iso",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ShadeMode {
    #[default]
    Density,
    Lines,
}

impl ShadeMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Density => Self::Lines,
            Self::Lines => Self::Density,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Density => "density",
            Self::Lines => "lines",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Density => "Classic brightness ramp ( .:-=+*#%@)",
            Self::Lines => "Line-only glyphs ( .'`-_/|\\)",
        }
    }
}

impl std::fmt::Display for ShadeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

const AFTER_HELP: &str = "\
Keys (interactive):
  arrows/hjkl  rotate    +/-  zoom    wasd  pan
  m  toggle shade        r  reset camera    q/Esc  quit

MCP:  stl2ascii mcp   (stdio; stdout is protocol — logs on stderr)
";

const LONG_ABOUT: &str = "\
ASCII STL 3D viewer — TUI, snapshot, or stdio MCP.

Features:
  • ASCII and binary STL
  • Framed ASCII 3D view with pan / zoom / rotate
  • Shade modes: density | lines
  • Status bar: camera + mesh stats (tris, AABB, area, volume)
  • --snapshot one-shot ASCII to the terminal
  • mcp subcommand: local stdio MCP (render_stl, mesh_stats, …)
";

#[derive(Debug, Parser)]
#[command(
    name = "stl2ascii",
    version,
    about = "ASCII STL viewer — TUI, snapshot, or stdio MCP",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to ASCII or binary STL
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Camera preset: front|back|left|right|top|bottom|iso
    #[arg(long, value_enum)]
    pub preset: Option<CameraPreset>,

    /// Camera yaw (degrees)
    #[arg(long)]
    pub yaw: Option<f32>,

    /// Camera pitch (degrees)
    #[arg(long)]
    pub pitch: Option<f32>,

    /// Zoom factor (>0; larger = closer)
    #[arg(long)]
    pub zoom: Option<f32>,

    /// Horizontal pan
    #[arg(long)]
    pub pan_x: Option<f32>,

    /// Vertical pan
    #[arg(long)]
    pub pan_y: Option<f32>,

    /// ASCII shade mode: density|lines
    #[arg(long, value_enum, default_value_t = ShadeMode::Density)]
    pub shade: ShadeMode,

    /// Render once to stdout and exit (leave ASCII in terminal)
    #[arg(long)]
    pub snapshot: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run local MCP server on stdio (stdout = protocol)
    Mcp,
}
