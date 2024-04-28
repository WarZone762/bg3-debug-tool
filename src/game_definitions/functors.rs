use game_object::GameObject;

use super::{Array, FixedString, MultiHashMap};

#[derive(GameObject)]
#[repr(C)]
pub(crate) struct Functors {
    vptr: *const (),
    pub functor_list: Array<*const ()>,
    pub functors_by_name: MultiHashMap<FixedString, *const ()>,
    pub next_functor_index: i32,
    pub unknown: i32,
    pub unique_name: FixedString,
}
