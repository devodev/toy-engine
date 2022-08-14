use cgmath::{Vector3, Vector4};

use crate::component;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GameObject {
    pub transform: component::Transform,
    pub color: component::Color,
}

impl GameObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_position(mut self, position: Vector3<f32>) -> Self {
        self.transform.position = position;
        self
    }

    pub fn with_rotation(mut self, rotation: Vector3<f32>) -> Self {
        self.transform.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vector3<f32>) -> Self {
        self.transform.scale = scale;
        self
    }

    pub fn with_color(mut self, color: Vector4<f32>) -> Self {
        self.color.color = color;
        self
    }
}
