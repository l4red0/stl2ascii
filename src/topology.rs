use std::collections::HashMap;

use glam::Vec3;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ComponentDetail {
    pub id: usize,
    pub triangles: usize,
    pub volume: f32,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub aabb_size: [f32; 3],
    pub center: [f32; 3],
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyStats {
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub components: usize,
    pub is_closed: bool,
    /// Genus for a closed single-component manifold; otherwise null.
    pub genus: Option<i32>,
    pub genus_notes: String,
    /// Top components by |volume| (at most 5).
    pub components_detail: Vec<ComponentDetail>,
    pub euler_v: usize,
    pub euler_e: usize,
    pub euler_f: usize,
}

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn union(&mut self, a: usize, b: usize) {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb {
            return;
        }
        if self.rank[ra] < self.rank[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        if self.rank[ra] == self.rank[rb] {
            self.rank[ra] += 1;
        }
    }
}

fn genus_notes(genus: Option<i32>, components: usize, is_closed: bool) -> String {
    match genus {
        Some(g) => format!("closed single-component manifold; genus = {g} (? = 2 - 2g)"),
        None if !is_closed => {
            "null because mesh is not a closed manifold (boundary or non-manifold edges); genus is defined only for a closed single-component manifold"
                .into()
        }
        None if components != 1 => format!(
            "null because mesh has {components} components (genus defined only for a closed single-component manifold)"
        ),
        None => "null because Euler characteristic is inconsistent with an integer genus".into(),
    }
}

fn component_details(
    vertices: &[Vec3],
    faces: &[[usize; 3]],
    uf: &mut UnionFind,
) -> Vec<ComponentDetail> {
    let mut by_root: HashMap<usize, Vec<usize>> = HashMap::new();
    for (fi, face) in faces.iter().enumerate() {
        let r = uf.find(face[0]);
        by_root.entry(r).or_default().push(fi);
    }

    let mut details = Vec::new();
    for (_root, face_ids) in by_root {
        let mut amin = Vec3::splat(f32::INFINITY);
        let mut amax = Vec3::splat(f32::NEG_INFINITY);
        let mut volume = 0.0_f32;
        for &fi in &face_ids {
            let [ia, ib, ic] = faces[fi];
            let v0 = vertices[ia];
            let v1 = vertices[ib];
            let v2 = vertices[ic];
            for v in [v0, v1, v2] {
                amin = amin.min(v);
                amax = amax.max(v);
            }
            volume += v0.dot(v1.cross(v2)) / 6.0;
        }
        if !amin.is_finite() {
            continue;
        }
        details.push(ComponentDetail {
            id: 0,
            triangles: face_ids.len(),
            volume,
            aabb_min: amin.to_array(),
            aabb_max: amax.to_array(),
            aabb_size: (amax - amin).to_array(),
            center: ((amin + amax) * 0.5).to_array(),
        });
    }
    details.sort_by(|a, b| {
        b.volume
            .abs()
            .partial_cmp(&a.volume.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, d) in details.iter_mut().enumerate() {
        d.id = i;
    }
    details.truncate(5);
    details
}

/// Edge census + components + optional genus from indexed triangle mesh.
pub fn compute(vertices: &[Vec3], faces: &[[usize; 3]]) -> TopologyStats {
    let vertex_count = vertices.len();
    let mut edge_faces: HashMap<(usize, usize), usize> = HashMap::new();
    let mut used = vec![false; vertex_count];
    let mut uf = UnionFind::new(vertex_count);

    for face in faces {
        let [a, b, c] = *face;
        for &v in &[a, b, c] {
            if v < used.len() {
                used[v] = true;
            }
        }
        for (i, j) in [(a, b), (b, c), (c, a)] {
            let key = if i < j { (i, j) } else { (j, i) };
            *edge_faces.entry(key).or_insert(0) += 1;
            if i < vertex_count && j < vertex_count {
                uf.union(i, j);
            }
        }
    }

    let mut boundary_edges = 0usize;
    let mut non_manifold_edges = 0usize;
    for &count in edge_faces.values() {
        if count == 1 {
            boundary_edges += 1;
        } else if count > 2 {
            non_manifold_edges += 1;
        }
    }

    let mut roots = std::collections::HashSet::new();
    for (i, &u) in used.iter().enumerate() {
        if u {
            roots.insert(uf.find(i));
        }
    }
    let components = roots.len();
    let components_detail = component_details(vertices, faces, &mut uf);

    let v = used.iter().filter(|&&u| u).count();
    let e = edge_faces.len();
    let f = faces.len();
    let is_closed = boundary_edges == 0 && non_manifold_edges == 0 && f > 0;

    let genus = if is_closed && components == 1 {
        let chi = v as i32 - e as i32 + f as i32;
        if (2 - chi) % 2 == 0 {
            Some((2 - chi) / 2)
        } else {
            None
        }
    } else {
        None
    };

    TopologyStats {
        boundary_edges,
        non_manifold_edges,
        components,
        is_closed,
        genus,
        genus_notes: genus_notes(genus, components, is_closed),
        components_detail,
        euler_v: v,
        euler_e: e,
        euler_f: f,
    }
}

/// Quantized weld for triangle soups (e.g. after hard clip). Round coords to 1e-5.
pub fn compute_from_positions(positions: &[[Vec3; 3]]) -> TopologyStats {
    let mut key_to_idx: HashMap<(i64, i64, i64), usize> = HashMap::new();
    let mut vertices = Vec::new();
    let mut faces = Vec::with_capacity(positions.len());
    let q = |x: f32| -> i64 { (x * 1e5).round() as i64 };

    for verts in positions {
        let mut idx = [0usize; 3];
        for (k, v) in verts.iter().enumerate() {
            let key = (q(v.x), q(v.y), q(v.z));
            let i = *key_to_idx.entry(key).or_insert_with(|| {
                let i = vertices.len();
                vertices.push(*v);
                i
            });
            idx[k] = i;
        }
        faces.push(idx);
    }
    compute(&vertices, &faces)
}
