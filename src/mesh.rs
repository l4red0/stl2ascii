use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    pub normal: Vec3,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeshStats {
    pub triangles: usize,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub aabb_size: [f32; 3],
    pub center: [f32; 3],
    pub surface_area: f32,
    pub volume: f32,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub stats: MeshStats,
    pub path: String,
}

fn unwrap_html_body(mut data: Vec<u8>) -> Vec<u8> {
    const PREFIX: &[u8] = b"<html><head></head><body>";
    const SUFFIX: &[u8] = b"</body></html>";
    if data.starts_with(PREFIX) {
        data.drain(..PREFIX.len());
        if data.ends_with(SUFFIX) {
            let n = data.len() - SUFFIX.len();
            data.truncate(n);
        }
    }
    data
}

impl Mesh {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read(path)
            .with_context(|| format!("open STL {}", path.display()))?;
        let data = unwrap_html_body(raw);
        let mut cursor = Cursor::new(data);
        let indexed = stl_io::read_stl(&mut cursor)
            .with_context(|| format!("parse STL {}", path.display()))?;

        let mut triangles = Vec::with_capacity(indexed.faces.len());
        let mut amin = Vec3::splat(f32::INFINITY);
        let mut amax = Vec3::splat(f32::NEG_INFINITY);
        let mut area = 0.0_f32;
        let mut volume = 0.0_f32;

        for face in &indexed.faces {
            let v0 = Vec3::from_array(indexed.vertices[face.vertices[0]].into());
            let v1 = Vec3::from_array(indexed.vertices[face.vertices[1]].into());
            let v2 = Vec3::from_array(indexed.vertices[face.vertices[2]].into());

            for v in [v0, v1, v2] {
                amin = amin.min(v);
                amax = amax.max(v);
            }

            let e1 = v1 - v0;
            let e2 = v2 - v0;
            let cross = e1.cross(e2);
            let cross_len = cross.length();
            area += 0.5 * cross_len;

            // Divergence theorem: V = 1/6 Σ det(v0,v1,v2)
            volume += v0.dot(v1.cross(v2)) / 6.0;

            let file_n = Vec3::from_array(face.normal.into());
            let normal = if file_n.length_squared() > 1e-12 {
                file_n.normalize()
            } else if cross_len > 1e-12 {
                cross / cross_len
            } else {
                Vec3::Z
            };

            triangles.push(Triangle {
                vertices: [v0, v1, v2],
                normal,
            });
        }

        if !amin.is_finite() {
            amin = Vec3::ZERO;
            amax = Vec3::ZERO;
        }

        let size = amax - amin;
        let center = (amin + amax) * 0.5;

        let stats = MeshStats {
            triangles: triangles.len(),
            aabb_min: amin.to_array(),
            aabb_max: amax.to_array(),
            aabb_size: size.to_array(),
            center: center.to_array(),
            surface_area: area,
            volume,
        };

        Ok(Self {
            triangles,
            stats,
            path: path.display().to_string(),
        })
    }

    pub fn center(&self) -> Vec3 {
        Vec3::from_array(self.stats.center)
    }

    pub fn max_extent(&self) -> f32 {
        let s = Vec3::from_array(self.stats.aabb_size);
        s.x.max(s.y).max(s.z).max(1e-6)
    }

    pub fn basename(&self) -> &str {
        Path::new(&self.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.path)
    }
}
