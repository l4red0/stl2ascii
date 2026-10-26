use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Axis-aligned box: min corner + size `[x, y, z, dx, dy, dz]` in model units.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AabbBox {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
}

impl AabbBox {
    pub fn from_slice(v: &[f32]) -> Result<Self, String> {
        if v.len() != 6 {
            return Err("box/clip must be [x,y,z,dx,dy,dz] (min corner + size)".into());
        }
        if v[3] <= 0.0 || v[4] <= 0.0 || v[5] <= 0.0 {
            return Err("box/clip sizes dx,dy,dz must be > 0".into());
        }
        Ok(Self {
            x: v[0],
            y: v[1],
            z: v[2],
            dx: v[3],
            dy: v[4],
            dz: v[5],
        })
    }

    pub fn min(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    pub fn max(&self) -> Vec3 {
        Vec3::new(self.x + self.dx, self.y + self.dy, self.z + self.dz)
    }

    pub fn contains_point(&self, p: Vec3) -> bool {
        let mn = self.min();
        let mx = self.max();
        p.x >= mn.x
            && p.x <= mx.x
            && p.y >= mn.y
            && p.y <= mx.y
            && p.z >= mn.z
            && p.z <= mx.z
    }
}
