use std::ops::Neg;

#[derive(Default)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub fn origin() -> Self {
        Self::default()
    }
}

#[derive(Default)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub fn i() -> Self {
        Vec3(1.0, 0.0, 0.0)
    }
    pub fn j() -> Self {
        Vec3(0.0, 1.0, 0.0)
    }
    pub fn k() -> Self {
        Vec3(0.0, 0.0, 1.0)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Self::Output {
        Self(-self.0, -self.1, -self.2)
    }
}
