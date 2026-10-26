use std::path::PathBuf;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig,
};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::aabb::AabbBox;
use crate::camera::{Camera, CameraArgs};
use crate::cli::{CameraPreset, ShadeMode};
use crate::diff::{mesh_diff, volume_in_box};
use crate::mcp_docs;
use crate::mesh::Mesh;
use crate::render::{
    render_deviation, render_framed, render_overlay, resolve_size,
};

#[derive(Clone, Default)]
pub struct StlMcp;

fn parse_preset(s: Option<&str>) -> Result<Option<CameraPreset>, String> {
    let Some(s) = s else {
        return Ok(None);
    };
    match s.to_ascii_lowercase().as_str() {
        "front" => Ok(Some(CameraPreset::Front)),
        "back" => Ok(Some(CameraPreset::Back)),
        "left" => Ok(Some(CameraPreset::Left)),
        "right" => Ok(Some(CameraPreset::Right)),
        "top" => Ok(Some(CameraPreset::Top)),
        "bottom" => Ok(Some(CameraPreset::Bottom)),
        "iso" => Ok(Some(CameraPreset::Iso)),
        other => Err(format!("unknown preset '{other}'")),
    }
}

fn parse_shade(s: Option<&str>) -> Result<ShadeMode, String> {
    match s.map(|s| s.to_ascii_lowercase()) {
        None => Ok(ShadeMode::Density),
        Some(ref s) if s == "density" => Ok(ShadeMode::Density),
        Some(ref s) if s == "lines" => Ok(ShadeMode::Lines),
        Some(other) => Err(format!("unknown shade '{other}'")),
    }
}

fn parse_box6(v: Option<Vec<f32>>) -> Result<Option<AabbBox>, String> {
    match v {
        None => Ok(None),
        Some(vals) => Ok(Some(AabbBox::from_slice(&vals)?)),
    }
}

fn parse_focus(v: Option<Vec<f32>>) -> Result<Option<[f32; 3]>, String> {
    match v {
        None => Ok(None),
        Some(v) if v.len() == 3 => Ok(Some([v[0], v[1], v[2]])),
        Some(_) => Err("focus must be [x,y,z]".into()),
    }
}

fn load_mesh(path: &str, clip: Option<AabbBox>) -> Result<Mesh, McpError> {
    let mesh = Mesh::load(PathBuf::from(path).as_path())
        .map_err(|e| McpError::invalid_params(format!("load STL: {e}"), None))?;
    Ok(match clip {
        Some(c) => mesh.clipped(c),
        None => mesh,
    })
}

fn json_ok(value: impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn camera_from(
    mesh: &Mesh,
    preset: Option<CameraPreset>,
    yaw: Option<f32>,
    pitch: Option<f32>,
    zoom: Option<f32>,
    pan_x: Option<f32>,
    pan_y: Option<f32>,
    focus: Option<[f32; 3]>,
    radius: Option<f32>,
    clip: Option<AabbBox>,
) -> Camera {
    Camera::from_mesh_and_args(
        mesh,
        &CameraArgs {
            preset,
            yaw,
            pitch,
            zoom,
            pan_x,
            pan_y,
            focus,
            radius,
            clip,
        },
    )
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct CamParams {
    #[serde(default)]
    #[schemars(description = "Camera preset name: front|back|left|right|top|bottom|iso.")]
    pub preset: Option<String>,
    #[serde(default)]
    #[schemars(description = "Yaw degrees. Overrides preset yaw when set.")]
    pub yaw: Option<f32>,
    #[serde(default)]
    #[schemars(description = "Pitch degrees. Overrides preset pitch when set.")]
    pub pitch: Option<f32>,
    #[serde(default)]
    #[schemars(description = "Zoom about orbit target; >1 closer. Default 1.")]
    pub zoom: Option<f32>,
    #[serde(default)]
    #[schemars(description = "Pan along camera right as fraction of fit_extent.")]
    pub pan_x: Option<f32>,
    #[serde(default)]
    #[schemars(description = "Pan along camera up as fraction of fit_extent.")]
    pub pan_y: Option<f32>,
    #[serde(default)]
    #[schemars(description = "Orbit target [x,y,z]. Overrides clip-center auto-fit when set.")]
    pub focus: Option<Vec<f32>>,
    #[serde(default)]
    #[schemars(description = "Orbit fit extent. Overrides clip-size auto-fit when set.")]
    pub radius: Option<f32>,
    #[serde(default)]
    #[schemars(description = "Inner width in characters. Alone → auto height from AABB aspect.")]
    pub width: Option<usize>,
    #[serde(default)]
    #[schemars(description = "Inner height in characters. Alone → auto width from AABB aspect.")]
    pub height: Option<usize>,
    #[serde(default)]
    #[schemars(description = "Hard-cut AABB [x,y,z,dx,dy,dz]. Auto-fits camera unless focus/radius set. Echoed on status line.")]
    pub clip: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenderParams {
    #[schemars(description = "Local filesystem path to ASCII or binary STL.")]
    pub path: String,
    #[serde(default)]
    #[schemars(description = "ASCII shade: density (default) or lines.")]
    pub shade: Option<String>,
    #[serde(flatten)]
    pub cam: CamParams,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OverlayParams {
    #[schemars(description = "Mesh A path.")]
    pub path_a: String,
    #[schemars(description = "Mesh B path.")]
    pub path_b: String,
    #[serde(flatten)]
    pub cam: CamParams,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeviationParams {
    #[schemars(description = "Display mesh path (rasterized surface).")]
    pub path: String,
    #[schemars(description = "Reference mesh path for unsigned distances.")]
    pub path_ref: String,
    #[serde(flatten)]
    pub cam: CamParams,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MeshStatsParams {
    #[schemars(description = "Local filesystem path to ASCII or binary STL.")]
    pub path: String,
    #[serde(default)]
    #[schemars(description = "Optional second mesh path. If set, response is {a,b,clip?} instead of flat MeshStats.")]
    pub path_b: Option<String>,
    #[serde(default)]
    #[schemars(description = "Hard-cut AABB [x,y,z,dx,dy,dz] before stats.")]
    pub clip: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MeshDiffParams {
    #[schemars(description = "Path to mesh A.")]
    pub path_a: String,
    #[schemars(description = "Path to mesh B.")]
    pub path_b: String,
    #[serde(default)]
    #[schemars(description = "Hard-cut both meshes and set volume_bounds to this box [x,y,z,dx,dy,dz].")]
    pub clip: Option<Vec<f32>>,
    #[serde(default)]
    #[schemars(description = "Surface samples per mesh (default 20000, clamp 100…200000).")]
    pub samples: Option<usize>,
    #[serde(default)]
    #[schemars(description = "Occupancy grid along longest volume_bounds axis (default 64, clamp 8…128).")]
    pub grid: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VolumeInBoxParams {
    #[schemars(description = "Local filesystem path to ASCII or binary STL.")]
    pub path: String,
    #[serde(rename = "box")]
    #[schemars(description = "Box [x,y,z,dx,dy,dz] min+size. JSON field name: box.")]
    pub r#box: Vec<f32>,
    #[serde(default)]
    #[schemars(description = "Grid along longest box axis (default 64, clamp 8…64). See estimated_error and grid_used.")]
    pub grid: Option<usize>,
}

fn prep_camera(
    mesh: &Mesh,
    preset: Option<String>,
    yaw: Option<f32>,
    pitch: Option<f32>,
    zoom: Option<f32>,
    pan_x: Option<f32>,
    pan_y: Option<f32>,
    focus: Option<Vec<f32>>,
    radius: Option<f32>,
    clip: Option<AabbBox>,
) -> Result<Camera, McpError> {
    let preset = parse_preset(preset.as_deref()).map_err(|e| McpError::invalid_params(e, None))?;
    let focus = parse_focus(focus).map_err(|e| McpError::invalid_params(e, None))?;
    Ok(camera_from(
        mesh, preset, yaw, pitch, zoom, pan_x, pan_y, focus, radius, clip,
    ))
}

#[tool_router]
impl StlMcp {
    #[tool(description = mcp_docs::RENDER_STL)]
    fn render_stl(
        &self,
        Parameters(p): Parameters<RenderParams>,
    ) -> Result<CallToolResult, McpError> {
        let clip = parse_box6(p.cam.clip.clone()).map_err(|e| McpError::invalid_params(e, None))?;
        let shade =
            parse_shade(p.shade.as_deref()).map_err(|e| McpError::invalid_params(e, None))?;
        let mesh = load_mesh(&p.path, clip)?;
        let c = &p.cam;
        let camera = prep_camera(
            &mesh,
            c.preset.clone(),
            c.yaw,
            c.pitch,
            c.zoom,
            c.pan_x,
            c.pan_y,
            c.focus.clone(),
            c.radius,
            clip,
        )?;
        let (w, h) = resolve_size(&mesh, c.width, c.height);
        let ascii = render_framed(&mesh, &camera, shade, w, h, clip);
        Ok(CallToolResult::success(vec![ContentBlock::text(ascii)]))
    }

    #[tool(description = mcp_docs::RENDER_OVERLAY)]
    fn render_overlay(
        &self,
        Parameters(p): Parameters<OverlayParams>,
    ) -> Result<CallToolResult, McpError> {
        let clip = parse_box6(p.cam.clip.clone()).map_err(|e| McpError::invalid_params(e, None))?;
        let a = load_mesh(&p.path_a, clip)?;
        let b = load_mesh(&p.path_b, clip)?;
        let c = &p.cam;
        let camera = prep_camera(
            &a,
            c.preset.clone(),
            c.yaw,
            c.pitch,
            c.zoom,
            c.pan_x,
            c.pan_y,
            c.focus.clone(),
            c.radius,
            clip,
        )?;
        let (w, h) = resolve_size(&a, c.width, c.height);
        let ascii = render_overlay(&a, &b, &camera, w, h, clip);
        Ok(CallToolResult::success(vec![ContentBlock::text(ascii)]))
    }

    #[tool(description = mcp_docs::RENDER_DEVIATION)]
    fn render_deviation(
        &self,
        Parameters(p): Parameters<DeviationParams>,
    ) -> Result<CallToolResult, McpError> {
        let clip = parse_box6(p.cam.clip.clone()).map_err(|e| McpError::invalid_params(e, None))?;
        let mesh = load_mesh(&p.path, clip)?;
        let href = load_mesh(&p.path_ref, clip)?;
        let c = &p.cam;
        let camera = prep_camera(
            &mesh,
            c.preset.clone(),
            c.yaw,
            c.pitch,
            c.zoom,
            c.pan_x,
            c.pan_y,
            c.focus.clone(),
            c.radius,
            clip,
        )?;
        let (w, h) = resolve_size(&mesh, c.width, c.height);
        let ascii = render_deviation(&mesh, &href, &camera, w, h, clip);
        Ok(CallToolResult::success(vec![ContentBlock::text(ascii)]))
    }

    #[tool(description = mcp_docs::MESH_STATS)]
    fn mesh_stats(
        &self,
        Parameters(p): Parameters<MeshStatsParams>,
    ) -> Result<CallToolResult, McpError> {
        let clip = parse_box6(p.clip).map_err(|e| McpError::invalid_params(e, None))?;
        let a = load_mesh(&p.path, clip)?;
        if let Some(path_b) = p.path_b {
            let b = load_mesh(&path_b, clip)?;
            let mut obj = json!({ "a": a.stats, "b": b.stats });
            if let Some(c) = clip {
                obj["clip"] = json!([c.x, c.y, c.z, c.dx, c.dy, c.dz]);
            }
            json_ok(obj)
        } else {
            json_ok(&a.stats)
        }
    }

    #[tool(description = mcp_docs::MESH_DIFF)]
    fn mesh_diff(
        &self,
        Parameters(p): Parameters<MeshDiffParams>,
    ) -> Result<CallToolResult, McpError> {
        let clip = parse_box6(p.clip).map_err(|e| McpError::invalid_params(e, None))?;
        let a = load_mesh(&p.path_a, clip)?;
        let b = load_mesh(&p.path_b, clip)?;
        let result = mesh_diff(
            &a,
            &b,
            p.samples.unwrap_or(20_000),
            p.grid.unwrap_or(64),
            clip,
        );
        json_ok(result)
    }

    #[tool(description = mcp_docs::VOLUME_IN_BOX)]
    fn volume_in_box(
        &self,
        Parameters(p): Parameters<VolumeInBoxParams>,
    ) -> Result<CallToolResult, McpError> {
        let b = AabbBox::from_slice(&p.r#box).map_err(|e| McpError::invalid_params(e, None))?;
        let mesh = load_mesh(&p.path, None)?;
        let result = volume_in_box(&mesh, b, p.grid.unwrap_or(64));
        json_ok(result)
    }

    #[tool(description = mcp_docs::LIST_PRESETS)]
    fn list_presets(&self) -> Result<CallToolResult, McpError> {
        let list: Vec<_> = CameraPreset::all()
            .iter()
            .map(|p| {
                let (yaw, pitch) = p.yaw_pitch();
                json!({ "name": p.to_string(), "yaw_deg": yaw, "pitch_deg": pitch })
            })
            .collect();
        json_ok(list)
    }

    #[tool(description = mcp_docs::LIST_SHADE_MODES)]
    fn list_shade_modes(&self) -> Result<CallToolResult, McpError> {
        json_ok([
            json!({
                "name": ShadeMode::Density.label(),
                "description": ShadeMode::Density.description()
            }),
            json!({
                "name": ShadeMode::Lines.label(),
                "description": ShadeMode::Lines.description()
            }),
        ])
    }
}

#[tool_handler(name = "stl2ascii", version = "0.1.0")]
impl ServerHandler for StlMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "stl2ascii",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(mcp_docs::SERVER_INSTRUCTIONS)
    }
}

pub async fn run_mcp() -> anyhow::Result<()> {
    let service = StlMcp.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
