use glam::Vec3;
use serde::Serialize;

use crate::mesh::Mesh;
use crate::spatial::TriGrid;

const SLICE_SEGMENT_CAP: usize = 5_000;
const ASCII_W: usize = 48;
const ASCII_H: usize = 24;

#[derive(Debug, Clone, Serialize)]
pub struct IsInsideResult {
    pub inside: bool,
    pub method: &'static str,
    pub point: [f32; 3],
    pub notes: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct SliceResult {
    pub axis: String,
    pub value: f32,
    pub segment_count: usize,
    pub segments: Vec<[[f32; 3]; 2]>,
    pub truncated: bool,
    pub ascii_preview: String,
    pub notes: &'static str,
}

pub fn is_inside(mesh: &Mesh, point: Vec3) -> IsInsideResult {
    let grid = TriGrid::build(mesh, 4096);
    IsInsideResult {
        inside: grid.inside(point),
        method: "raycast_parity",
        point: point.to_array(),
        notes: "Raycast parity along +X through triangle faces. Approximate; open/non-manifold meshes are unreliable. Optional clip hard-cuts the mesh before the test.",
    }
}

/// Axis: 0=x, 1=y, 2=z.
pub fn slice_mesh(mesh: &Mesh, axis: usize, value: f32) -> SliceResult {
    let mut segments = Vec::new();
    let mut truncated = false;
    for tri in &mesh.triangles {
        if let Some(seg) = triangle_plane_segment(&tri.vertices, axis, value) {
            if segments.len() >= SLICE_SEGMENT_CAP {
                truncated = true;
                break;
            }
            segments.push([seg.0.to_array(), seg.1.to_array()]);
        }
    }
    let axis_name = match axis {
        0 => "x",
        1 => "y",
        _ => "z",
    };
    let ascii_preview = ascii_section(&segments, axis);
    SliceResult {
        axis: axis_name.into(),
        value,
        segment_count: segments.len(),
        segments,
        truncated,
        ascii_preview,
        notes: "Plane ∩ triangles as line segments. Optional clip hard-cuts before slicing. ascii_preview is a compact ortho plot in the two free axes.",
    }
}

pub fn parse_axis(s: &str) -> Result<usize, String> {
    match s.to_ascii_lowercase().as_str() {
        "x" => Ok(0),
        "y" => Ok(1),
        "z" => Ok(2),
        other => Err(format!("axis must be x|y|z, got '{other}'")),
    }
}

fn component(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn triangle_plane_segment(verts: &[Vec3; 3], axis: usize, value: f32) -> Option<(Vec3, Vec3)> {
    let mut hits = Vec::with_capacity(3);
    for i in 0..3 {
        let a = verts[i];
        let b = verts[(i + 1) % 3];
        let ca = component(a, axis);
        let cb = component(b, axis);
        if (ca - value).abs() < 1e-9 && (cb - value).abs() < 1e-9 {
            // Edge lies in plane — take endpoints
            hits.push(a);
            hits.push(b);
        } else if (ca - value) * (cb - value) < 0.0
            || ((ca - value).abs() < 1e-9)
            || ((cb - value).abs() < 1e-9)
        {
            if (ca - value).abs() < 1e-9 {
                hits.push(a);
            } else if (cb - value).abs() < 1e-9 {
                hits.push(b);
            } else {
                let t = (value - ca) / (cb - ca);
                hits.push(a + (b - a) * t);
            }
        }
    }
    // Dedup near-identical hits
    let mut uniq: Vec<Vec3> = Vec::new();
    for h in hits {
        if !uniq.iter().any(|u| (*u - h).length_squared() < 1e-12) {
            uniq.push(h);
        }
    }
    if uniq.len() >= 2 {
        Some((uniq[0], uniq[1]))
    } else {
        None
    }
}

fn free_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2), // y,z
        1 => (0, 2), // x,z
        _ => (0, 1), // x,y
    }
}

fn ascii_section(segments: &[[[f32; 3]; 2]], cut_axis: usize) -> String {
    if segments.is_empty() {
        return "(no segments)".into();
    }
    let (ax0, ax1) = free_axes(cut_axis);
    let mut amin = [f32::INFINITY; 2];
    let mut amax = [f32::NEG_INFINITY; 2];
    for seg in segments {
        for p in seg {
            let u = p[ax0];
            let v = p[ax1];
            amin[0] = amin[0].min(u);
            amin[1] = amin[1].min(v);
            amax[0] = amax[0].max(u);
            amax[1] = amax[1].max(v);
        }
    }
    let mut size = [amax[0] - amin[0], amax[1] - amin[1]];
    if size[0] < 1e-6 {
        size[0] = 1.0;
    }
    if size[1] < 1e-6 {
        size[1] = 1.0;
    }
    let pad = 0.05;
    amin[0] -= size[0] * pad;
    amin[1] -= size[1] * pad;
    amax[0] += size[0] * pad;
    amax[1] += size[1] * pad;
    size = [amax[0] - amin[0], amax[1] - amin[1]];

    let mut buf = vec![vec![' '; ASCII_W]; ASCII_H];
    let to_px = |u: f32, v: f32| -> (isize, isize) {
        let x = ((u - amin[0]) / size[0] * (ASCII_W - 1) as f32).round() as isize;
        let y = ((1.0 - (v - amin[1]) / size[1]) * (ASCII_H - 1) as f32).round() as isize;
        (x, y)
    };
    for seg in segments {
        let (x0, y0) = to_px(seg[0][ax0], seg[0][ax1]);
        let (x1, y1) = to_px(seg[1][ax0], seg[1][ax1]);
        plot_line(&mut buf, x0, y0, x1, y1);
    }

    let labels = match cut_axis {
        0 => ("y→", "z↑"),
        1 => ("x→", "z↑"),
        _ => ("x→", "y↑"),
    };
    let mut out = format!("slice ascii {} {} ({}x{})\n", labels.0, labels.1, ASCII_W, ASCII_H);
    for row in buf {
        out.push_str(&row.iter().collect::<String>());
        out.push('\n');
    }
    out
}

fn plot_line(buf: &mut [Vec<char>], x0: isize, y0: isize, x1: isize, y1: isize) {
    let mut x0 = x0;
    let mut y0 = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 && (x0 as usize) < ASCII_W && (y0 as usize) < ASCII_H {
            buf[y0 as usize][x0 as usize] = '#';
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
