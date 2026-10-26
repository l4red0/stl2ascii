use glam::{Mat4, Vec3};

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

impl Camera {
    pub fn from_mesh_and_args(
        mesh: &Mesh,
        preset: Option<CameraPreset>,
        yaw: Option<f32>,
        pitch: Option<f32>,
        zoom: Option<f32>,
        pan_x: Option<f32>,
        pan_y: Option<f32>,
    ) -> Self {
        let (pyaw, ppitch) = preset.unwrap_or(CameraPreset::Iso).yaw_pitch();
        Self {
            yaw: yaw.unwrap_or(pyaw),
            pitch: pitch.unwrap_or(ppitch),
            zoom: zoom.unwrap_or(1.0).max(0.01),
            pan_x: pan_x.unwrap_or(0.0),
            pan_y: pan_y.unwrap_or(0.0),
            target: mesh.center(),
            fit_extent: mesh.max_extent() * 0.65,
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

    pub fn view_matrix(&self) -> Mat4 {
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
}
