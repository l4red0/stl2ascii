use glam::{Mat4, Vec3};

use crate::aabb::AabbBox;
use crate::cli::CameraPreset;
use crate::mesh::Mesh;

#[derive(Debug, Clone)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub target: Vec3,
    pub fit_extent: f32,
}

pub struct CameraArgs {
    pub preset: Option<CameraPreset>,
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub zoom: Option<f32>,
    pub pan_x: Option<f32>,
    pub pan_y: Option<f32>,
    pub focus: Option<[f32; 3]>,
    pub radius: Option<f32>,
    /// When set and focus/radius omitted, orbit fits this clip box.
    pub clip: Option<AabbBox>,
}

impl Camera {
    pub fn from_mesh_and_args(mesh: &Mesh, args: &CameraArgs) -> Self {
        let (pyaw, ppitch) = args.preset.unwrap_or(CameraPreset::Iso).yaw_pitch();
        let (default_target, default_fit) = if let Some(clip) = args.clip {
            let c = (clip.min() + clip.max()) * 0.5;
            let fit = clip.dx.max(clip.dy).max(clip.dz).max(1e-6) * 0.65;
            (c, fit)
        } else {
            (mesh.center(), mesh.max_extent() * 0.65)
        };
        let target = args
            .focus
            .map(Vec3::from_array)
            .unwrap_or(default_target);
        let fit_extent = args
            .radius
            .filter(|r| *r > 0.0)
            .unwrap_or(default_fit);
        Self {
            yaw: args.yaw.unwrap_or(pyaw),
            pitch: args.pitch.unwrap_or(ppitch),
            zoom: args.zoom.unwrap_or(1.0).max(0.01),
            pan_x: args.pan_x.unwrap_or(0.0),
            pan_y: args.pan_y.unwrap_or(0.0),
            target,
            fit_extent,
        }
    }

    pub fn reset_from(&mut self, initial: &Camera) {
        *self = initial.clone();
    }

    pub fn forward(&self) -> Vec3 {
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();
        Vec3::new(
            pitch.cos() * yaw.sin(),
            pitch.sin(),
            pitch.cos() * yaw.cos(),
        )
        .normalize()
    }

    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = self.forward();
        let world_up = Vec3::Y;
        let right = {
            let r = forward.cross(world_up);
            if r.length_squared() > 1e-8 {
                r.normalize()
            } else {
                Vec3::X
            }
        };
        let up = right.cross(forward).normalize();
        (forward, right, up)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let (forward, right, up) = self.basis();
        let eye = self.target - forward * (self.fit_extent * 2.0);
        let pan = right * self.pan_x * self.fit_extent + up * self.pan_y * self.fit_extent;
        let eye = eye + pan;
        let target = self.target + pan;
        glam::camera::rh::view::look_at_mat4(eye, target, up)
    }

    /// Orthographic half-extent in world units (before aspect).
    pub fn ortho_half_extent(&self) -> f32 {
        self.fit_extent / self.zoom
    }

    /// World-space AABB of the orthographic view volume (for status / framing checks).
    pub fn view_aabb(&self, aspect: f32) -> ([f32; 3], [f32; 3]) {
        let (forward, right, up) = self.basis();
        let pan = right * self.pan_x * self.fit_extent + up * self.pan_y * self.fit_extent;
        let center = self.target + pan;
        let half = self.ortho_half_extent();
        let half_w = half * aspect;
        let half_h = half;
        let corners = [
            center + right * half_w + up * half_h,
            center + right * half_w - up * half_h,
            center - right * half_w + up * half_h,
            center - right * half_w - up * half_h,
        ];
        let mut amin = corners[0];
        let mut amax = corners[0];
        for c in &corners[1..] {
            amin = amin.min(*c);
            amax = amax.max(*c);
        }
        // Expand slightly along view axis for a readable 3D AABB
        amin -= forward.abs() * half * 0.01;
        amax += forward.abs() * half * 0.01;
        (amin.to_array(), amax.to_array())
    }
}
