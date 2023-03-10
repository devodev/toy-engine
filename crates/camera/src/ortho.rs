use cgmath::{Matrix4, SquareMatrix};

use super::Camera;

#[derive(Debug, Copy, Clone)]
pub struct CameraOrthographic {
    width: u32,
    height: u32,
    aspect_ratio: f32,

    zoom_base: f32,
    zoom: f32,
    near: f32,
    far: f32,

    proj: Matrix4<f32>,
}

// One of the most common matrices used for orthographic projection can be
// defined by a 6-tuple, (left, right, bottom, top, near, far), which defines
// the clipping planes. These planes form a box with the minimum corner at
// (left, bottom, -near) and the maximum corner at (right, top, -far).
#[allow(unused)]
impl CameraOrthographic {
    pub fn new(width: u32, height: u32) -> Self {
        let mut camera = Self { ..Self::default() };
        camera.resize(width, height);
        camera
    }

    fn compute_projection_matrix(&mut self) {
        self.proj = cgmath::ortho(
            -self.aspect_ratio * self.zoom,
            self.aspect_ratio * self.zoom,
            -self.zoom,
            self.zoom,
            self.near,
            self.far,
        )
    }
}

impl Camera for CameraOrthographic {
    fn projection_matrix(&self) -> Matrix4<f32> {
        self.proj
    }

    fn set_zoom(&mut self, amount: f32) {
        self.zoom = amount;
        if self.zoom < 0.1 {
            self.zoom = 0.1;
        }
        self.compute_projection_matrix()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn reset_zoom(&mut self) {
        self.zoom = self.zoom_base;
        self.compute_projection_matrix()
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.aspect_ratio = width as f32 / height as f32;
        self.compute_projection_matrix()
    }
}

impl Default for CameraOrthographic {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            aspect_ratio: 0.0,
            zoom_base: 1.0,
            zoom: 1.0,
            near: 0.1,
            far: 10.0,
            proj: Matrix4::identity(),
        }
    }
}
