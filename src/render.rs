use glam::Vec3;

use crate::aabb::AabbBox;
use crate::camera::Camera;
use crate::cli::ShadeMode;
use crate::mesh::Mesh;
use crate::shade::shade_char;
use crate::spatial::TriGrid;

/// Character cell aspect (terminal glyphs are taller than wide).
pub const CHAR_ASPECT: f32 = 2.0;

const OVERLAY_A: char = '#';
const OVERLAY_B: char = '%';
const OVERLAY_BOTH: char = '*';

struct DepthBuf {
    width: usize,
    height: usize,
    depth: Vec<f32>,
    filled: Vec<bool>,
    /// Optional world position at filled cells (for deviation).
    world: Vec<Option<Vec3>>,
    inten: Vec<f32>,
}

impl DepthBuf {
    fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        Self {
            width,
            height,
            depth: vec![f32::NEG_INFINITY; n],
            filled: vec![false; n],
            world: vec![None; n],
            inten: vec![0.0; n],
        }
    }

    fn rasterize_shaded(&mut self, mesh: &Mesh, camera: &Camera, store_world: bool) {
        let view = camera.view_matrix();
        let half = camera.ortho_half_extent();
        let aspect = (self.width as f32 / CHAR_ASPECT) / self.height as f32;
        let half_w = half * aspect;
        let half_h = half;
        let light = (-camera.forward() + Vec3::Y * 0.35).normalize();
        let (forward, right, up) = camera.basis();
        let pan = right * camera.pan_x * camera.fit_extent + up * camera.pan_y * camera.fit_extent;
        let center = camera.target + pan;

        for tri in &mesh.triangles {
            let ndotl = tri.normal.dot(light);
            if ndotl <= 0.0 {
                continue;
            }
            let intensity = ndotl.clamp(0.0, 1.0);
            let p0 = view.transform_point3(tri.vertices[0]);
            let p1 = view.transform_point3(tri.vertices[1]);
            let p2 = view.transform_point3(tri.vertices[2]);

            let to_screen = |p: Vec3| -> (f32, f32, f32) {
                let sx = (p.x / half_w * 0.5 + 0.5) * self.width as f32;
                let sy = (1.0 - (p.y / half_h * 0.5 + 0.5)) * self.height as f32;
                (sx, sy, p.z)
            };
            let (x0, y0, z0) = to_screen(p0);
            let (x1, y1, z1) = to_screen(p1);
            let (x2, y2, z2) = to_screen(p2);

            let min_x = x0.min(x1).min(x2).floor().max(0.0) as i32;
            let max_x = x0.max(x1).max(x2).ceil().min(self.width as f32 - 1.0) as i32;
            let min_y = y0.min(y1).min(y2).floor().max(0.0) as i32;
            let max_y = y0.max(y1).max(y2).ceil().min(self.height as f32 - 1.0) as i32;
            let area = edge(x0, y0, x1, y1, x2, y2);
            if area.abs() < 1e-8 {
                continue;
            }

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let px = x as f32 + 0.5;
                    let py = y as f32 + 0.5;
                    let w0 = edge(x1, y1, x2, y2, px, py) / area;
                    let w1 = edge(x2, y2, x0, y0, px, py) / area;
                    let w2 = edge(x0, y0, x1, y1, px, py) / area;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * z0 + w1 * z1 + w2 * z2;
                    let idx = y as usize * self.width + x as usize;
                    if z >= self.depth[idx] {
                        self.depth[idx] = z;
                        self.inten[idx] = intensity;
                        self.filled[idx] = true;
                        if store_world {
                            let world = tri.vertices[0] * w0
                                + tri.vertices[1] * w1
                                + tri.vertices[2] * w2;
                            self.world[idx] = Some(world);
                            let _ = (center, forward); // silence if unused later
                        }
                    }
                }
            }
        }
    }

    /// Fill-only raster (no shading); records presence for overlay masks.
    fn rasterize_mask(mesh: &Mesh, camera: &Camera, width: usize, height: usize) -> (Vec<f32>, Vec<bool>) {
        let mut depth = vec![f32::NEG_INFINITY; width * height];
        let mut filled = vec![false; width * height];
        let view = camera.view_matrix();
        let half = camera.ortho_half_extent();
        let aspect = (width as f32 / CHAR_ASPECT) / height as f32;
        let half_w = half * aspect;
        let half_h = half;

        for tri in &mesh.triangles {
            if tri.normal.dot(-camera.forward()) <= 0.0 && tri.normal.dot(-camera.forward() + Vec3::Y * 0.35) <= 0.0
            {
                // keep same backface as shaded path
            }
            let ndotl = tri.normal.dot((-camera.forward() + Vec3::Y * 0.35).normalize());
            if ndotl <= 0.0 {
                continue;
            }
            let p0 = view.transform_point3(tri.vertices[0]);
            let p1 = view.transform_point3(tri.vertices[1]);
            let p2 = view.transform_point3(tri.vertices[2]);
            let to_screen = |p: Vec3| -> (f32, f32, f32) {
                let sx = (p.x / half_w * 0.5 + 0.5) * width as f32;
                let sy = (1.0 - (p.y / half_h * 0.5 + 0.5)) * height as f32;
                (sx, sy, p.z)
            };
            let (x0, y0, z0) = to_screen(p0);
            let (x1, y1, z1) = to_screen(p1);
            let (x2, y2, z2) = to_screen(p2);
            let min_x = x0.min(x1).min(x2).floor().max(0.0) as i32;
            let max_x = x0.max(x1).max(x2).ceil().min(width as f32 - 1.0) as i32;
            let min_y = y0.min(y1).min(y2).floor().max(0.0) as i32;
            let max_y = y0.max(y1).max(y2).ceil().min(height as f32 - 1.0) as i32;
            let area = edge(x0, y0, x1, y1, x2, y2);
            if area.abs() < 1e-8 {
                continue;
            }
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let px = x as f32 + 0.5;
                    let py = y as f32 + 0.5;
                    let w0 = edge(x1, y1, x2, y2, px, py) / area;
                    let w1 = edge(x2, y2, x0, y0, px, py) / area;
                    let w2 = edge(x0, y0, x1, y1, px, py) / area;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * z0 + w1 * z1 + w2 * z2;
                    let idx = y as usize * width + x as usize;
                    if z >= depth[idx] {
                        depth[idx] = z;
                        filled[idx] = true;
                    }
                }
            }
        }
        (depth, filled)
    }
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}

/// Glyph per distance bucket (matches `deviation_legend`).
fn dist_to_glyph(d: f32) -> char {
    // .≤0.01 :≤0.05 -≤0.1 =≤0.5 +≤1 #%>1
    if d <= 0.01 {
        '.'
    } else if d <= 0.05 {
        ':'
    } else if d <= 0.1 {
        '-'
    } else if d <= 0.5 {
        '='
    } else if d <= 1.0 {
        '+'
    } else {
        '#'
    }
}

pub fn render_grid(
    mesh: &Mesh,
    camera: &Camera,
    shade: ShadeMode,
    width: usize,
    height: usize,
) -> Vec<String> {
    let width = width.max(1);
    let height = height.max(1);
    let mut buf = DepthBuf::new(width, height);
    buf.rasterize_shaded(mesh, camera, false);
    let mut lines = Vec::with_capacity(height);
    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            let idx = y * width + x;
            if buf.filled[idx] {
                row.push(shade_char(shade, buf.inten[idx]));
            } else {
                row.push(' ');
            }
        }
        lines.push(row);
    }
    lines
}

fn clip_status_suffix(clip: Option<AabbBox>) -> String {
    match clip {
        Some(c) => format!(
            " | clip:[{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}]",
            c.x, c.y, c.z, c.dx, c.dy, c.dz
        ),
        None => String::new(),
    }
}

/// Truncate `body` so `body + suffix` fits `width`, keeping `suffix` (e.g. clip) intact.
fn truncate_keep_suffix(body: &str, suffix: &str, width: usize) -> String {
    if width == 0 || body.len() + suffix.len() <= width {
        return format!("{body}{suffix}");
    }
    let keep = width.saturating_sub(suffix.len()).saturating_sub(1);
    let mut truncated: String = body.chars().take(keep).collect();
    truncated.push('…');
    truncated.push_str(suffix);
    truncated
}

pub fn format_status(
    mesh: &Mesh,
    camera: &Camera,
    shade: ShadeMode,
    width: usize,
    height: usize,
    clip: Option<AabbBox>,
) -> String {
    let aspect = (width.max(1) as f32 / CHAR_ASPECT) / height.max(1) as f32;
    let (vmin, vmax) = camera.view_aabb(aspect);
    let s = &mesh.stats;
    let body = format!(
        "{} | {} | yaw:{:.0} pitch:{:.0} zoom:{:.2} | tris:{} AABB:{:.1}x{:.1}x{:.1} area:{:.1} vol:{:.1} closed:{} | view:[{:.1},{:.1},{:.1}]-[{:.1},{:.1},{:.1}]",
        mesh.basename(),
        shade.label(),
        camera.yaw,
        camera.pitch,
        camera.zoom,
        s.triangles,
        s.aabb_size[0],
        s.aabb_size[1],
        s.aabb_size[2],
        s.surface_area,
        s.volume,
        s.is_closed,
        vmin[0],
        vmin[1],
        vmin[2],
        vmax[0],
        vmax[1],
        vmax[2],
    );
    truncate_keep_suffix(&body, &clip_status_suffix(clip), width)
}

/// Numeric bucket → glyph legend for deviation status (matches dist_to_glyph).
pub fn deviation_legend() -> &'static str {
    " .≤0.01 :≤0.05 -≤0.1 =≤0.5 +≤1 #%>1"
}

fn frame_lines(grid: &[String], status: &str) -> String {
    let inner_w = grid.first().map(|r| r.len()).unwrap_or(0);
    let mut out = String::new();
    let border = format!("+{}+", "-".repeat(inner_w));
    out.push_str(&border);
    out.push('\n');
    for row in grid {
        out.push('|');
        out.push_str(row);
        out.push('|');
        out.push('\n');
    }
    out.push_str(&border);
    out.push('\n');
    out.push_str(status);
    out.push('\n');
    out
}

pub fn render_framed(
    mesh: &Mesh,
    camera: &Camera,
    shade: ShadeMode,
    inner_w: usize,
    inner_h: usize,
    clip: Option<AabbBox>,
) -> String {
    let grid = render_grid(mesh, camera, shade, inner_w, inner_h);
    let status = format_status(mesh, camera, shade, inner_w + 2, inner_h, clip);
    frame_lines(&grid, &status)
}

pub fn render_overlay(
    mesh_a: &Mesh,
    mesh_b: &Mesh,
    camera: &Camera,
    inner_w: usize,
    inner_h: usize,
    clip: Option<AabbBox>,
) -> String {
    let w = inner_w.max(1);
    let h = inner_h.max(1);
    let (_da, fa) = DepthBuf::rasterize_mask(mesh_a, camera, w, h);
    let (_db, fb) = DepthBuf::rasterize_mask(mesh_b, camera, w, h);
    let mut grid = Vec::with_capacity(h);
    for y in 0..h {
        let mut row = String::with_capacity(w);
        for x in 0..w {
            let idx = y * w + x;
            let a = fa[idx];
            let b = fb[idx];
            row.push(match (a, b) {
                (true, true) => OVERLAY_BOTH,
                (true, false) => OVERLAY_A,
                (false, true) => OVERLAY_B,
                (false, false) => ' ',
            });
        }
        grid.push(row);
    }
    let aspect = (w as f32 / CHAR_ASPECT) / h as f32;
    let (vmin, vmax) = camera.view_aabb(aspect);
    let body = format!(
        "{}+# {}+% both=* | yaw:{:.0} pitch:{:.0} zoom:{:.2} | view:[{:.1},{:.1},{:.1}]-[{:.1},{:.1},{:.1}]",
        mesh_a.basename(),
        mesh_b.basename(),
        camera.yaw,
        camera.pitch,
        camera.zoom,
        vmin[0],
        vmin[1],
        vmin[2],
        vmax[0],
        vmax[1],
        vmax[2],
    );
    let status = truncate_keep_suffix(&body, &clip_status_suffix(clip), w + 2);
    frame_lines(&grid, &status)
}

pub fn render_deviation(
    mesh: &Mesh,
    mesh_ref: &Mesh,
    camera: &Camera,
    inner_w: usize,
    inner_h: usize,
    clip: Option<AabbBox>,
) -> String {
    let w = inner_w.max(1);
    let h = inner_h.max(1);
    let mut buf = DepthBuf::new(w, h);
    buf.rasterize_shaded(mesh, camera, true);
    let grid_ref = TriGrid::build(mesh_ref, 4096);
    let mut grid = Vec::with_capacity(h);
    for y in 0..h {
        let mut row = String::with_capacity(w);
        for x in 0..w {
            let idx = y * w + x;
            if let Some(world) = buf.world[idx] {
                let d = grid_ref.nearest_dist(world);
                row.push(dist_to_glyph(d));
            } else {
                row.push(' ');
            }
        }
        grid.push(row);
    }
    let body = format!(
        "{} vs {} |{} | yaw:{:.0} pitch:{:.0} zoom:{:.2}",
        mesh.basename(),
        mesh_ref.basename(),
        deviation_legend(),
        camera.yaw,
        camera.pitch,
        camera.zoom,
    );
    let status = truncate_keep_suffix(&body, &clip_status_suffix(clip), w + 2);
    frame_lines(&grid, &status)
}

pub fn default_snapshot_size() -> (usize, usize) {
    (100, 40)
}

pub fn resolve_size(
    mesh: &Mesh,
    width: Option<usize>,
    height: Option<usize>,
) -> (usize, usize) {
    let (dw, dh) = default_snapshot_size();
    match (width, height) {
        (Some(w), Some(h)) => (w.max(8), h.max(4)),
        (Some(w), None) => {
            let w = w.max(8);
            let size = Vec3::from_array(mesh.stats.aabb_size);
            let aspect = (size.x.max(1e-6) / size.y.max(1e-6)) * CHAR_ASPECT;
            let h = ((w as f32) / aspect).round().max(4.0) as usize;
            (w, h)
        }
        (None, Some(h)) => {
            let h = h.max(4);
            let size = Vec3::from_array(mesh.stats.aabb_size);
            let aspect = (size.x.max(1e-6) / size.y.max(1e-6)) * CHAR_ASPECT;
            let w = ((h as f32) * aspect).round().max(8.0) as usize;
            (w, h)
        }
        (None, None) => (dw, dh),
    }
}
