use glam::Vec3;

use crate::aabb::AabbBox;
use crate::mesh::Triangle;

/// Clip a triangle against an AABB; return zero or more triangles inside the box.
pub fn clip_triangle_aabb(tri: &Triangle, box_: AabbBox) -> Vec<Triangle> {
    let mut poly: Vec<Vec3> = tri.vertices.to_vec();
    let mn = box_.min();
    let mx = box_.max();

    poly = clip_plane(poly, |p| p.x >= mn.x - 1e-7, |a, b| lerp_axis(a, b, 0, mn.x));
    poly = clip_plane(poly, |p| p.x <= mx.x + 1e-7, |a, b| lerp_axis(a, b, 0, mx.x));
    poly = clip_plane(poly, |p| p.y >= mn.y - 1e-7, |a, b| lerp_axis(a, b, 1, mn.y));
    poly = clip_plane(poly, |p| p.y <= mx.y + 1e-7, |a, b| lerp_axis(a, b, 1, mx.y));
    poly = clip_plane(poly, |p| p.z >= mn.z - 1e-7, |a, b| lerp_axis(a, b, 2, mn.z));
    poly = clip_plane(poly, |p| p.z <= mx.z + 1e-7, |a, b| lerp_axis(a, b, 2, mx.z));

    // Force verts into the box so floating error cannot leave geometry outside.
    for p in &mut poly {
        *p = clamp_into_box(*p, mn, mx);
    }

    if poly.len() < 3 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(poly.len().saturating_sub(2));
    let n = tri.normal;
    for i in 1..poly.len().saturating_sub(1) {
        let v0 = poly[0];
        let v1 = poly[i];
        let v2 = poly[i + 1];
        let cross = (v1 - v0).cross(v2 - v0);
        if cross.length_squared() < 1e-16 {
            continue;
        }
        let normal = if n.length_squared() > 1e-12 {
            n
        } else {
            cross.normalize()
        };
        out.push(Triangle {
            vertices: [v0, v1, v2],
            normal,
        });
    }
    out
}

fn clamp_into_box(p: Vec3, mn: Vec3, mx: Vec3) -> Vec3 {
    Vec3::new(
        p.x.clamp(mn.x, mx.x),
        p.y.clamp(mn.y, mx.y),
        p.z.clamp(mn.z, mx.z),
    )
}

fn lerp_axis(a: Vec3, b: Vec3, axis: usize, val: f32) -> Vec3 {
    let (va, vb) = match axis {
        0 => (a.x, b.x),
        1 => (a.y, b.y),
        _ => (a.z, b.z),
    };
    let t = if (vb - va).abs() < 1e-12 {
        0.0
    } else {
        (val - va) / (vb - va)
    };
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn clip_plane(
    input: Vec<Vec3>,
    inside: impl Fn(Vec3) -> bool,
    intersect: impl Fn(Vec3, Vec3) -> Vec3,
) -> Vec<Vec3> {
    if input.is_empty() {
        return input;
    }
    let mut output = Vec::new();
    let n = input.len();
    for i in 0..n {
        let cur = input[i];
        let prev = input[(i + n - 1) % n];
        let cur_in = inside(cur);
        let prev_in = inside(prev);
        if cur_in {
            if !prev_in {
                output.push(intersect(prev, cur));
            }
            output.push(cur);
        } else if prev_in {
            output.push(intersect(prev, cur));
        }
    }
    output
}
