use crate::util::Vec3;

use super::Geometry;

pub struct Sphere {
    certre: Vec3,
    radius: f32,
}

impl Geometry for Sphere {}
