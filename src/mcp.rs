use std::path::PathBuf;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::camera::Camera;
use crate::cli::{CameraPreset, ShadeMode};
use crate::mesh::Mesh;
use crate::render::{default_snapshot_size, render_framed};

#[derive(Clone, Default)]
pub struct StlMcp;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenderParams {
    /// Local filesystem path to ASCII or binary STL
    pub path: String,
    /// Camera preset: front|back|left|right|top|bottom|iso
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub yaw: Option<f32>,
    #[serde(default)]
    pub pitch: Option<f32>,
    /// Zoom factor (>0; larger = closer)
    #[serde(default)]
    pub zoom: Option<f32>,
    #[serde(default)]
    pub pan_x: Option<f32>,
    #[serde(default)]
    pub pan_y: Option<f32>,
    /// Shade mode: density|lines
    #[serde(default)]
    pub shade: Option<String>,
    #[serde(default)]
    pub width: Option<usize>,
    #[serde(default)]
    pub height: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PathParams {
    /// Local filesystem path to ASCII or binary STL
    pub path: String,
}

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

#[tool_router]
impl StlMcp {
    #[tool(
        description = "Render an STL file to bordered ASCII art with a status line (camera, shade, mesh stats). Paths are local filesystem paths."
    )]
    fn render_stl(
        &self,
        Parameters(p): Parameters<RenderParams>,
    ) -> Result<CallToolResult, McpError> {
        let preset =
            parse_preset(p.preset.as_deref()).map_err(|e| McpError::invalid_params(e, None))?;
        let shade =
            parse_shade(p.shade.as_deref()).map_err(|e| McpError::invalid_params(e, None))?;

        let mesh = Mesh::load(PathBuf::from(&p.path).as_path())
            .map_err(|e| McpError::invalid_params(format!("load STL: {e}"), None))?;
        let camera = Camera::from_mesh_and_args(
            &mesh, preset, p.yaw, p.pitch, p.zoom, p.pan_x, p.pan_y,
        );
        let (dw, dh) = default_snapshot_size();
        let w = p.width.unwrap_or(dw).max(8);
        let h = p.height.unwrap_or(dh).max(4);
        let ascii = render_framed(&mesh, &camera, shade, w, h);
        Ok(CallToolResult::success(vec![ContentBlock::text(ascii)]))
    }

    #[tool(description = "Compute mesh statistics for an STL (triangles, AABB, area, volume, center).")]
    fn mesh_stats(
        &self,
        Parameters(p): Parameters<PathParams>,
    ) -> Result<CallToolResult, McpError> {
        let mesh = Mesh::load(PathBuf::from(&p.path).as_path())
            .map_err(|e| McpError::invalid_params(format!("load STL: {e}"), None))?;
        let text = serde_json::to_string_pretty(&mesh.stats).unwrap_or_else(|_| "{}".into());
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }

    #[tool(description = "List camera presets with yaw/pitch degrees.")]
    fn list_presets(&self) -> Result<CallToolResult, McpError> {
        let list: Vec<_> = CameraPreset::all()
            .iter()
            .map(|p| {
                let (yaw, pitch) = p.yaw_pitch();
                json!({ "name": p.to_string(), "yaw": yaw, "pitch": pitch })
            })
            .collect();
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".into()),
        )]))
    }

    #[tool(description = "List ASCII shade modes (density, lines).")]
    fn list_shade_modes(&self) -> Result<CallToolResult, McpError> {
        let list = [
            json!({
                "name": ShadeMode::Density.label(),
                "description": ShadeMode::Density.description()
            }),
            json!({
                "name": ShadeMode::Lines.label(),
                "description": ShadeMode::Lines.description()
            }),
        ];
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".into()),
        )]))
    }
}

#[tool_handler(
    name = "stl2ascii",
    version = "0.1.0",
    instructions = "ASCII STL viewer MCP. Tools: render_stl (path + optional preset/yaw/pitch/zoom/pan/shade/width/height → bordered ASCII), mesh_stats(path), list_presets, list_shade_modes. Paths are local filesystem paths. Stateless: each call reloads the file."
)]
impl ServerHandler for StlMcp {}

pub async fn run_mcp() -> anyhow::Result<()> {
    let service = StlMcp.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
