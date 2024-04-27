use game_object::GameObject;

#[derive(GameObject)]
#[repr(C)]
pub(crate) struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(GameObject)]
#[repr(C)]
pub(crate) struct Quat {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
