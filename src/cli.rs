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
Keys: arrows/hjkl rotate  +/- zoom  wasd pan  m shade  r reset  q quit
MCP:  stl2ascii mcp   (prefer MCP for full tool docs)
zoom>1 closer; pan is fraction of fit extent; --focus x,y,z --radius R
clip/box: x,y,z,dx,dy,dz (min + size)
";

const LONG_ABOUT: &str = "\
ASCII STL viewer + mesh QA (TUI, snapshot, stdio MCP).

Features: ASCII/binary STL; hard clip; mesh_stats; mesh_diff; volume_in_box;
overlay; deviation render; focus. Primary interface: `stl2ascii mcp`.
";

#[derive(Debug, Parser)]
#[command(
    name = "stl2ascii",
    version,
    about = "ASCII STL viewer + mesh QA (MCP-first)",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to ASCII or binary STL
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Camera preset
    #[arg(long, value_enum)]
    pub preset: Option<CameraPreset>,

    /// Yaw degrees
    #[arg(long)]
    pub yaw: Option<f32>,

    /// Pitch degrees
    #[arg(long)]
    pub pitch: Option<f32>,

    /// Zoom (>1 closer about orbit target)
    #[arg(long)]
    pub zoom: Option<f32>,

    /// Pan right (fraction of fit extent)
    #[arg(long)]
    pub pan_x: Option<f32>,

    /// Pan up (fraction of fit extent)
    #[arg(long)]
    pub pan_y: Option<f32>,

    /// Orbit target x,y,z
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    pub focus: Option<Vec<f32>>,

    /// Orbit fit extent (world units)
    #[arg(long)]
    pub radius: Option<f32>,

    /// Shade: density|lines
    #[arg(long, value_enum, default_value_t = ShadeMode::Density)]
    pub shade: ShadeMode,

    /// Clip box x,y,z,dx,dy,dz
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    pub clip: Option<Vec<f32>>,

    /// Snapshot to stdout and exit
    #[arg(long)]
    pub snapshot: bool,

    /// Snapshot width (chars)
    #[arg(long)]
    pub width: Option<usize>,

    /// Snapshot height (chars)
    #[arg(long)]
    pub height: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Stdio MCP server (primary interface)
    Mcp,
    /// Compare two meshes (histogram + outside volumes)
    Diff {
        a: PathBuf,
        b: PathBuf,
        /// Clip x,y,z,dx,dy,dz
        #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
        clip: Option<Vec<f32>>,
        #[arg(long, default_value_t = 20_000)]
        samples: usize,
        #[arg(long, default_value_t = 64)]
        grid: usize,
    },
    /// Volume of mesh ∩ box
    VolumeInBox {
        file: PathBuf,
        /// Box x,y,z,dx,dy,dz
        #[arg(long = "box", value_delimiter = ',', allow_hyphen_values = true)]
        box_vals: Vec<f32>,
        #[arg(long, default_value_t = 64)]
        grid: usize,
    },
    /// Print mesh_stats JSON (optional second file for side-by-side)
    Stats {
        file: PathBuf,
        /// Optional second mesh
        #[arg(long = "b")]
        file_b: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
        clip: Option<Vec<f32>>,
    },
    /// Overlay two meshes (#=A %=B *=both)
    Overlay {
        a: PathBuf,
        b: PathBuf,
        #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
        clip: Option<Vec<f32>>,
        #[arg(long)]
        width: Option<usize>,
        #[arg(long)]
        height: Option<usize>,
        #[arg(long, value_enum)]
        preset: Option<CameraPreset>,
        #[arg(long)]
        yaw: Option<f32>,
        #[arg(long)]
        pitch: Option<f32>,
        #[arg(long)]
        zoom: Option<f32>,
    },
    /// Deviation-coded surface vs reference mesh
    Deviation {
        file: PathBuf,
        /// Reference mesh for distances
        #[arg(long = "ref")]
        reference: PathBuf,
        #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
        clip: Option<Vec<f32>>,
        #[arg(long)]
        width: Option<usize>,
        #[arg(long)]
        height: Option<usize>,
        #[arg(long, value_enum)]
        preset: Option<CameraPreset>,
        #[arg(long)]
        yaw: Option<f32>,
        #[arg(long)]
        pitch: Option<f32>,
        #[arg(long)]
        zoom: Option<f32>,
    },
}
