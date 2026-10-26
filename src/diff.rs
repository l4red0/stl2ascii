use glam::Vec3;
use serde::Serialize;

use crate::aabb::AabbBox;
use crate::mesh::Mesh;
use crate::spatial::{sample_surface, TriGrid};

pub const HIST_LABELS: [&str; 6] = ["<=0.01", "<=0.05", "<=0.1", "<=0.5", "<=1", ">1"];
pub const HIST_EDGES: [f32; 5] = [0.01, 0.05, 0.1, 0.5, 1.0];

/// Max grid along longest axis for volume_in_box (128³ occupancy is too slow for typical meshes).
pub const VOLUME_IN_BOX_GRID_MAX: usize = 64;
pub const MESH_DIFF_GRID_MAX: usize = 128;

#[derive(Debug, Clone, Serialize)]
pub struct MeshDiffResult {
    pub method: &'static str,
    pub notes: &'static str,
    pub samples_per_side: usize,
    pub histogram_a_to_b: Vec<HistBucket>,
    pub histogram_b_to_a: Vec<HistBucket>,
    pub volume_a_outside_b: f32,
    pub volume_b_outside_a: f32,
    /// Bounds used for occupancy volumes: clip box when clip was set, else union mesh AABB.
    pub volume_bounds: [f32; 6],
    pub voxel_size: f32,
    pub grid_dims: [usize; 3],
    pub units: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistBucket {
    pub bucket: &'static str,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeInBoxResult {
    pub method: &'static str,
    pub notes: &'static str,
    pub volume: f32,
    pub surface_area_in_box: f32,
    /// Discretization scale ≈ one voxel volume (model units³); use to choose grid.
    pub estimated_error: f32,
    pub voxel_size: f32,
    pub grid_dims: [usize; 3],
    pub grid_requested: usize,
    pub grid_used: usize,
    pub box_spec: [f32; 6],
    pub units: &'static str,
}

fn hist_counts(distances: &[f32]) -> Vec<HistBucket> {
    let mut counts = [0usize; 6];
    for &d in distances {
        let mut placed = false;
        for (i, &edge) in HIST_EDGES.iter().enumerate() {
            if d <= edge {
                counts[i] += 1;
                placed = true;
                break;
            }
        }
        if !placed {
            counts[5] += 1;
        }
    }
    HIST_LABELS
        .iter()
        .zip(counts)
        .map(|(label, count)| HistBucket {
            bucket: label,
            count,
        })
        .collect()
}

fn union_aabb(a: &Mesh, b: &Mesh) -> (Vec3, Vec3) {
    let amin = Vec3::from_array(a.stats.aabb_min).min(Vec3::from_array(b.stats.aabb_min));
    let amax = Vec3::from_array(a.stats.aabb_max).max(Vec3::from_array(b.stats.aabb_max));
    (amin, amax)
}

fn voxel_dims(extent: Vec3, grid: usize, grid_max: usize) -> (f32, [usize; 3]) {
    let grid = grid.clamp(8, grid_max);
    let max_e = extent.max_element().max(1e-6);
    let voxel = max_e / grid as f32;
    let dims = [
        ((extent.x / voxel).ceil() as usize).clamp(1, grid_max),
        ((extent.y / voxel).ceil() as usize).clamp(1, grid_max),
        ((extent.z / voxel).ceil() as usize).clamp(1, grid_max),
    ];
    (voxel, dims)
}

fn occupancy_outside(
    grid_a: &TriGrid,
    grid_b: &TriGrid,
    origin: Vec3,
    voxel: f32,
    dims: [usize; 3],
) -> (f32, f32) {
    let mut a_out = 0usize;
    let mut b_out = 0usize;
    let vox_vol = voxel * voxel * voxel;
    for iz in 0..dims[2] {
        for iy in 0..dims[1] {
            for ix in 0..dims[0] {
                let p = origin
                    + Vec3::new(
                        (ix as f32 + 0.5) * voxel,
                        (iy as f32 + 0.5) * voxel,
                        (iz as f32 + 0.5) * voxel,
                    );
                let in_a = grid_a.inside(p);
                let in_b = grid_b.inside(p);
                if in_a && !in_b {
                    a_out += 1;
                } else if in_b && !in_a {
                    b_out += 1;
                }
            }
        }
    }
    (a_out as f32 * vox_vol, b_out as f32 * vox_vol)
}

/// Compare meshes. When `volume_clip` is set, outside volumes are integrated only inside that box
/// (triangle filter for histograms may still use a looser triangle-AABB cull separately).
pub fn mesh_diff(
    mesh_a: &Mesh,
    mesh_b: &Mesh,
    samples: usize,
    grid: usize,
    volume_clip: Option<AabbBox>,
) -> MeshDiffResult {
    let samples = samples.clamp(100, 200_000);
    let grid_a = TriGrid::build(mesh_a, 4096);
    let grid_b = TriGrid::build(mesh_b, 4096);

    let pts_a = sample_surface(mesh_a, samples);
    let pts_b = sample_surface(mesh_b, samples);
    let dist_ab: Vec<f32> = pts_a.iter().map(|p| grid_b.nearest_dist(*p)).collect();
    let dist_ba: Vec<f32> = pts_b.iter().map(|p| grid_a.nearest_dist(*p)).collect();

    let (amin, amax, bounds6, notes) = if let Some(clip) = volume_clip {
        (
            clip.min(),
            clip.max(),
            [clip.x, clip.y, clip.z, clip.dx, clip.dy, clip.dz],
            "Distances are unsigned closest-point on hard-clipped meshes. Outside volumes are voxel occupancy ONLY inside volume_bounds (= the clip box). Prefer volume_in_box for tiny slabs. Raycast parity is approximate; open/non-manifold meshes are unreliable.",
        )
    } else {
        let (amin, amax) = union_aabb(mesh_a, mesh_b);
        let size = amax - amin;
        (
            amin,
            amax,
            [amin.x, amin.y, amin.z, size.x, size.y, size.z],
            "Distances are unsigned closest-point to the other mesh. Outside volumes use raycast parity over volume_bounds (union mesh AABB). Approximate; open/non-manifold meshes are unreliable. Pass clip for hard-cut meshes and clip-scoped volumes, or call volume_in_box.",
        )
    };

    let extent = (amax - amin).max(Vec3::splat(1e-6));
    let (voxel, dims) = voxel_dims(extent, grid, MESH_DIFF_GRID_MAX);
    let (vol_a_out, vol_b_out) = occupancy_outside(&grid_a, &grid_b, amin, voxel, dims);

    MeshDiffResult {
        method: "surface_distance_histogram+voxel_occupancy",
        notes,
        samples_per_side: samples,
        histogram_a_to_b: hist_counts(&dist_ab),
        histogram_b_to_a: hist_counts(&dist_ba),
        volume_a_outside_b: vol_a_out,
        volume_b_outside_a: vol_b_out,
        volume_bounds: bounds6,
        voxel_size: voxel,
        grid_dims: dims,
        units: "model_units_as_in_stl",
    }
}

pub fn volume_in_box(mesh: &Mesh, b: AabbBox, grid: usize) -> VolumeInBoxResult {
    let requested = grid;
    let used = requested.clamp(8, VOLUME_IN_BOX_GRID_MAX);
    let grid_mesh = TriGrid::build(mesh, 4096);
    let amin = b.min();
    let amax = b.max();
    let extent = (amax - amin).max(Vec3::splat(1e-6));
    let (voxel, dims) = voxel_dims(extent, used, VOLUME_IN_BOX_GRID_MAX);
    let mut inside = 0usize;
    let vox_vol = voxel * voxel * voxel;
    for iz in 0..dims[2] {
        for iy in 0..dims[1] {
            for ix in 0..dims[0] {
                let p = amin
                    + Vec3::new(
                        (ix as f32 + 0.5) * voxel,
                        (iy as f32 + 0.5) * voxel,
                        (iz as f32 + 0.5) * voxel,
                    );
                if grid_mesh.inside(p) {
                    inside += 1;
                }
            }
        }
    }

    VolumeInBoxResult {
        method: "voxel_occupancy+centroid_area",
        notes: "volume is voxel occupancy of mesh ∩ box (raycast parity; approximate). estimated_error is one voxel volume (voxel_size³) — the discretization scale for choosing grid. surface_area_in_box sums triangle area when the triangle centroid lies inside the box: area is triangle-granular and NOT comparable across tessellation densities. grid is clamped to 8…64 (default 64; higher often times out on ~10k+ tris). See grid_requested vs grid_used.",
        volume: inside as f32 * vox_vol,
        surface_area_in_box: mesh.surface_area_in_box(b),
        estimated_error: vox_vol,
        voxel_size: voxel,
        grid_dims: dims,
        grid_requested: requested,
        grid_used: used,
        box_spec: [b.x, b.y, b.z, b.dx, b.dy, b.dz],
        units: "model_units_as_in_stl",
    }
}
