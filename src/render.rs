use glam::Vec3;

use crate::camera::Camera;
use crate::cli::ShadeMode;
use crate::mesh::Mesh;
use crate::shade::shade_char;

/// Character cell aspect (terminal glyphs are taller than wide).
const CHAR_ASPECT: f32 = 2.0;

pub fn render_grid(mesh: &Mesh, camera: &Camera, shade: ShadeMode, width: usize, height: usize) -> Vec<String> {
    let width = width.max(1);
    let height = height.max(1);

    let view = camera.view_matrix();
    let half = camera.ortho_half_extent();
    let aspect = (width as f32 / CHAR_ASPECT) / height as f32;
    let half_w = half * aspect;
    let half_h = half;

    let light = (-camera.forward() + Vec3::Y * 0.35).normalize();

    let mut depth = vec![f32::NEG_INFINITY; width * height];
    let mut inten = vec![0.0_f32; width * height];
    let mut filled = vec![false; width * height];

    for tri in &mesh.triangles {
        let n_world = tri.normal;
        let ndotl = n_world.dot(light);
        if ndotl <= 0.0 {
            continue; // back-face / unlit
        }
        let intensity = ndotl.clamp(0.0, 1.0);

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
                // RH look-at: camera looks down -Z, nearer = larger z (less negative)
                if z >= depth[idx] {
                    depth[idx] = z;
                    inten[idx] = intensity;
                    filled[idx] = true;
                }
            }
        }
    }

    let mut lines = Vec::with_capacity(height);
    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            let idx = y * width + x;
            if filled[idx] {
                row.push(shade_char(shade, inten[idx]));
            } else {
                row.push(' ');
            }
        }
        lines.push(row);
    }
    lines
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}

pub fn format_status(mesh: &Mesh, camera: &Camera, shade: ShadeMode, width: usize) -> String {
    let s = &mesh.stats;
    let line = format!(
        "{} | {} | yaw:{:.0} pitch:{:.0} zoom:{:.2} | tris:{} AABB:{:.1}x{:.1}x{:.1} area:{:.1} vol:{:.1}",
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
    );
    if width == 0 || line.len() <= width {
        line
    } else {
        let mut truncated = line.chars().take(width.saturating_sub(1)).collect::<String>();
        truncated.push('…');
        truncated
    }
}

pub fn render_framed(
    mesh: &Mesh,
    camera: &Camera,
    shade: ShadeMode,
    inner_w: usize,
    inner_h: usize,
) -> String {
    let grid = render_grid(mesh, camera, shade, inner_w, inner_h);
    let mut out = String::new();
    let border = format!("+{}+", "-".repeat(inner_w));
    out.push_str(&border);
    out.push('\n');
    for row in &grid {
        out.push('|');
        out.push_str(row);
        out.push('|');
        out.push('\n');
    }
    out.push_str(&border);
    out.push('\n');
    out.push_str(&format_status(mesh, camera, shade, inner_w + 2));
    out.push('\n');
    out
}

pub fn default_snapshot_size() -> (usize, usize) {
    (100, 40)
}
