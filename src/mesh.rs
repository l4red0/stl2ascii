use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use serde::Serialize;

use crate::aabb::AabbBox;
use crate::topology::{self, ComponentDetail, TopologyStats};

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
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub components: usize,
    pub is_closed: bool,
    pub genus: Option<i32>,
    pub genus_notes: String,
    /// Top 5 connected components by |volume|.
    pub components_detail: Vec<ComponentDetail>,
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

fn tri_normal(v0: Vec3, v1: Vec3, v2: Vec3, file_n: Vec3) -> Vec3 {
    let cross = (v1 - v0).cross(v2 - v0);
    let cross_len = cross.length();
    if file_n.length_squared() > 1e-12 {
        file_n.normalize()
    } else if cross_len > 1e-12 {
        cross / cross_len
    } else {
        Vec3::Z
    }
}

fn build_stats(triangles: &[Triangle], topo: &TopologyStats) -> MeshStats {
    let mut amin = Vec3::splat(f32::INFINITY);
    let mut amax = Vec3::splat(f32::NEG_INFINITY);
    let mut area = 0.0_f32;
    let mut volume = 0.0_f32;

    for tri in triangles {
        let [v0, v1, v2] = tri.vertices;
        for v in [v0, v1, v2] {
            amin = amin.min(v);
            amax = amax.max(v);
        }
        area += 0.5 * (v1 - v0).cross(v2 - v0).length();
        volume += v0.dot(v1.cross(v2)) / 6.0;
    }

    if !amin.is_finite() {
        amin = Vec3::ZERO;
        amax = Vec3::ZERO;
    }

    MeshStats {
        triangles: triangles.len(),
        aabb_min: amin.to_array(),
        aabb_max: amax.to_array(),
        aabb_size: (amax - amin).to_array(),
        center: ((amin + amax) * 0.5).to_array(),
        surface_area: area,
        volume,
        boundary_edges: topo.boundary_edges,
        non_manifold_edges: topo.non_manifold_edges,
        components: topo.components,
        is_closed: topo.is_closed,
        genus: topo.genus,
        genus_notes: topo.genus_notes.clone(),
        components_detail: topo.components_detail.clone(),
    }
}

impl Mesh {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read(path).with_context(|| format!("open STL {}", path.display()))?;
        let data = unwrap_html_body(raw);
        let mut cursor = Cursor::new(data);
        let indexed =
            stl_io::read_stl(&mut cursor).with_context(|| format!("parse STL {}", path.display()))?;

        let vertices: Vec<Vec3> = indexed
            .vertices
            .iter()
            .map(|v| Vec3::from_array((*v).into()))
            .collect();

        let mut faces = Vec::with_capacity(indexed.faces.len());
        let mut triangles = Vec::with_capacity(indexed.faces.len());

        for face in &indexed.faces {
            let idx = [
                face.vertices[0],
                face.vertices[1],
                face.vertices[2],
            ];
            faces.push(idx);
            let v0 = vertices[idx[0]];
            let v1 = vertices[idx[1]];
            let v2 = vertices[idx[2]];
            let file_n = Vec3::from_array(face.normal.into());
            triangles.push(Triangle {
                vertices: [v0, v1, v2],
                normal: tri_normal(v0, v1, v2, file_n),
            });
        }

        // Weld by exact STL vertex indices (not float hash).
        let topo = topology::compute(&vertices, &faces);
        let stats = build_stats(&triangles, &topo);

        Ok(Self {
            triangles,
            stats,
            path: path.display().to_string(),
        })
    }

    /// Hard-cut triangles to an AABB (Sutherland–Hodgman). Stats/AABB reflect clipped geometry.
    pub fn clipped(&self, clip: AabbBox) -> Self {
        let mut triangles = Vec::new();
        for tri in &self.triangles {
            triangles.extend(crate::clip::clip_triangle_aabb(tri, clip));
        }
        let positions: Vec<[Vec3; 3]> = triangles.iter().map(|t| t.vertices).collect();
        let topo = topology::compute_from_positions(&positions);
        let stats = build_stats(&triangles, &topo);
        Self {
            triangles,
            stats,
            path: self.path.clone(),
        }
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

    /// Surface area of triangles whose centroid lies inside `box` (v1 approximation).
    pub fn surface_area_in_box(&self, b: AabbBox) -> f32 {
        let mut area = 0.0;
        for tri in &self.triangles {
            let [v0, v1, v2] = tri.vertices;
            let c = (v0 + v1 + v2) / 3.0;
            if b.contains_point(c) {
                area += 0.5 * (v1 - v0).cross(v2 - v0).length();
            }
        }
        area
    }
}
