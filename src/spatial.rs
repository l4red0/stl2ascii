use glam::Vec3;

use crate::mesh::{Mesh, Triangle};

/// Uniform grid over triangle AABBs for nearest-point and raycast queries.
pub struct TriGrid {
    origin: Vec3,
    cell: f32,
    dims: [usize; 3],
    cells: Vec<Vec<usize>>,
    tris: Vec<Triangle>,
}

impl TriGrid {
    pub fn build(mesh: &Mesh, target_cells: usize) -> Self {
        let amin = Vec3::from_array(mesh.stats.aabb_min);
        let amax = Vec3::from_array(mesh.stats.aabb_max);
        let extent = (amax - amin).max(Vec3::splat(1e-6));
        let n = mesh.triangles.len().max(1);
        let cells_est = target_cells.max(8) as f32;
        let vol = extent.x * extent.y * extent.z;
        let cell = (vol / cells_est).cbrt().max(extent.max_element() / 64.0).max(1e-6);

        let dims = [
            ((extent.x / cell).ceil() as usize).clamp(1, 128),
            ((extent.y / cell).ceil() as usize).clamp(1, 128),
            ((extent.z / cell).ceil() as usize).clamp(1, 128),
        ];
        let origin = amin - Vec3::splat(cell * 0.01);
        let mut cells = vec![Vec::new(); dims[0] * dims[1] * dims[2]];
        let tris = mesh.triangles.clone();

        for (ti, tri) in tris.iter().enumerate() {
            let tmin = tri.vertices[0].min(tri.vertices[1]).min(tri.vertices[2]);
            let tmax = tri.vertices[0].max(tri.vertices[1]).max(tri.vertices[2]);
            let i0 = self_clamp(((tmin.x - origin.x) / cell).floor() as isize, dims[0]);
            let i1 = self_clamp(((tmax.x - origin.x) / cell).floor() as isize, dims[0]);
            let j0 = self_clamp(((tmin.y - origin.y) / cell).floor() as isize, dims[1]);
            let j1 = self_clamp(((tmax.y - origin.y) / cell).floor() as isize, dims[1]);
            let k0 = self_clamp(((tmin.z - origin.z) / cell).floor() as isize, dims[2]);
            let k1 = self_clamp(((tmax.z - origin.z) / cell).floor() as isize, dims[2]);
            for ix in i0..=i1 {
                for iy in j0..=j1 {
                    for iz in k0..=k1 {
                        let idx = ix + dims[0] * (iy + dims[1] * iz);
                        cells[idx].push(ti);
                    }
                }
            }
        }

        let _ = n;
        Self {
            origin,
            cell,
            dims,
            cells,
            tris,
        }
    }

    fn cell_index(&self, p: Vec3) -> Option<usize> {
        let ix = ((p.x - self.origin.x) / self.cell).floor() as isize;
        let iy = ((p.y - self.origin.y) / self.cell).floor() as isize;
        let iz = ((p.z - self.origin.z) / self.cell).floor() as isize;
        if ix < 0
            || iy < 0
            || iz < 0
            || ix as usize >= self.dims[0]
            || iy as usize >= self.dims[1]
            || iz as usize >= self.dims[2]
        {
            return None;
        }
        Some(ix as usize + self.dims[0] * (iy as usize + self.dims[1] * iz as usize))
    }

    pub fn nearest_dist(&self, p: Vec3) -> f32 {
        let mut best = f32::INFINITY;
        // Search expanding rings of cells
        let cx = ((p.x - self.origin.x) / self.cell).floor() as i32;
        let cy = ((p.y - self.origin.y) / self.cell).floor() as i32;
        let cz = ((p.z - self.origin.z) / self.cell).floor() as i32;
        let max_r = self.dims[0].max(self.dims[1]).max(self.dims[2]) as i32;

        for r in 0..=max_r {
            let mut found = false;
            for dx in -r..=r {
                for dy in -r..=r {
                    for dz in -r..=r {
                        if r > 0 && dx.abs().max(dy.abs()).max(dz.abs()) != r {
                            continue;
                        }
                        let ix = cx + dx;
                        let iy = cy + dy;
                        let iz = cz + dz;
                        if ix < 0
                            || iy < 0
                            || iz < 0
                            || ix as usize >= self.dims[0]
                            || iy as usize >= self.dims[1]
                            || iz as usize >= self.dims[2]
                        {
                            continue;
                        }
                        let idx = ix as usize
                            + self.dims[0] * (iy as usize + self.dims[1] * iz as usize);
                        for &ti in &self.cells[idx] {
                            let d = point_triangle_distance(p, &self.tris[ti]);
                            if d < best {
                                best = d;
                                found = true;
                            }
                        }
                    }
                }
            }
            if found && best < (r as f32 + 1.0) * self.cell {
                break;
            }
        }

        if !best.is_finite() {
            // Fallback brute force
            for tri in &self.tris {
                best = best.min(point_triangle_distance(p, tri));
            }
        }
        best
    }

    /// Odd/even raycast along +X for point-in-mesh (approximate; open meshes are unreliable).
    pub fn inside(&self, p: Vec3) -> bool {
        let dir = Vec3::X;
        let mut hits = 0u32;
        // Collect candidate tris from cells along the ray, then unique-test.
        let mut candidates = Vec::new();
        let mut t = 0.0_f32;
        let max_t = self.dims[0] as f32 * self.cell * 2.0 + self.cell;
        while t <= max_t {
            let q = p + dir * t;
            if let Some(ci) = self.cell_index(q) {
                candidates.extend_from_slice(&self.cells[ci]);
            }
            t += self.cell;
        }
        candidates.sort_unstable();
        candidates.dedup();
        if candidates.is_empty() {
            for tri in &self.tris {
                if let Some(th) = ray_triangle(p, dir, tri) {
                    if th >= 1e-6 {
                        hits += 1;
                    }
                }
            }
        } else {
            for ti in candidates {
                if let Some(th) = ray_triangle(p, dir, &self.tris[ti]) {
                    if th >= 1e-6 {
                        hits += 1;
                    }
                }
            }
        }
        hits % 2 == 1
    }
}

fn self_clamp(v: isize, dim: usize) -> usize {
    if v < 0 {
        0
    } else if v as usize >= dim {
        dim - 1
    } else {
        v as usize
    }
}

fn point_triangle_distance(p: Vec3, tri: &Triangle) -> f32 {
    let a = tri.vertices[0];
    let b = tri.vertices[1];
    let c = tri.vertices[2];
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return p.distance(a);
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return p.distance(b);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return p.distance(a + ab * v);
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return p.distance(c);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return p.distance(a + ac * w);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return p.distance(b + (c - b) * w);
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    p.distance(a + ab * v + ac * w)
}

fn ray_triangle(orig: Vec3, dir: Vec3, tri: &Triangle) -> Option<f32> {
    let v0 = tri.vertices[0];
    let v1 = tri.vertices[1];
    let v2 = tri.vertices[2];
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let pvec = dir.cross(e2);
    let det = e1.dot(pvec);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = orig - v0;
    let u = tvec.dot(pvec) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(e1);
    let v = dir.dot(qvec) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(qvec) * inv;
    if t >= 0.0 {
        Some(t)
    } else {
        None
    }
}

/// Sample points on mesh surface (area-weighted), capped at `max_samples`.
pub fn sample_surface(mesh: &Mesh, max_samples: usize) -> Vec<Vec3> {
    let max_samples = max_samples.max(1);
    let mut areas = Vec::with_capacity(mesh.triangles.len());
    let mut total = 0.0_f32;
    for tri in &mesh.triangles {
        let a = 0.5
            * (tri.vertices[1] - tri.vertices[0])
                .cross(tri.vertices[2] - tri.vertices[0])
                .length();
        areas.push(a);
        total += a;
    }
    if total <= 1e-12 {
        return mesh
            .triangles
            .iter()
            .take(max_samples)
            .map(|t| (t.vertices[0] + t.vertices[1] + t.vertices[2]) / 3.0)
            .collect();
    }

    let mut out = Vec::with_capacity(max_samples);
    // Deterministic LCG for reproducible samples
    let mut state: u64 = 0xC0FFEE;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 33) as f32 / (u32::MAX as f32)
    };

    for _ in 0..max_samples {
        let mut r = next() * total;
        let mut ti = 0usize;
        for (i, &a) in areas.iter().enumerate() {
            if r <= a {
                ti = i;
                break;
            }
            r -= a;
            ti = i;
        }
        let tri = &mesh.triangles[ti];
        // Uniform barycentric
        let mut u = next();
        let mut v = next();
        if u + v > 1.0 {
            u = 1.0 - u;
            v = 1.0 - v;
        }
        let w = 1.0 - u - v;
        out.push(tri.vertices[0] * w + tri.vertices[1] * u + tri.vertices[2] * v);
    }
    out
}
