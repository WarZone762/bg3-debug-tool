#[derive(Debug)]
#[repr(C)]
pub(crate) struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Quat {
    w: f32,
    x: f32,
    y: f32,
    z: f32,
}
