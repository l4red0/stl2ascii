use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TopologyStats {
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub components: usize,
    pub is_closed: bool,
    /// Genus for a closed single-component manifold; otherwise null.
    pub genus: Option<i32>,
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

/// Edge census + components + optional genus from indexed triangle mesh.
pub fn compute(vertex_count: usize, faces: &[[usize; 3]]) -> TopologyStats {
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

    let v = used.iter().filter(|&&u| u).count();
    let e = edge_faces.len();
    let f = faces.len();
    let is_closed = boundary_edges == 0 && non_manifold_edges == 0 && f > 0;

    let genus = if is_closed && components == 1 {
        let chi = v as i32 - e as i32 + f as i32;
        // χ = 2 - 2g  =>  g = (2 - χ) / 2
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
        euler_v: v,
        euler_e: e,
        euler_f: f,
    }
}
