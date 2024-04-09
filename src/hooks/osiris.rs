use std::ptr;

use windows::{
    core::{s, w},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::{
    err,
    game_definitions::{GamePtr, OsiArgumentDesc, OsirisStaticGlobals},
    globals::Globals,
    hook_definitions,
};

hook_definitions! {
osiris("Osiris.dll") {
    #[symbol_name = "?RegisterDIVFunctions@COsiris@@QEAAXPEAUTOsirisInitFunction@@@Z"]
    fn RegisterDivFunctions(a: *const u8, b: *const u8) -> i32 {
        unsafe {
            let osiris_module = LoadLibraryW(w!("Osiris.dll")).unwrap();
            let osiris_ctor_proc =
                GetProcAddress(osiris_module, s!("??0COsiris@@QEAA@XZ")).unwrap() as _;

            Globals::osiris_globals_set(find_osiris_globals(osiris_ctor_proc));

            HOOKS.Call.set(*(b as *const *const ()).add(1));
            HOOKS.Query.set(*(b as *const *const ()).add(2));

            original::RegisterDivFunctions(a, b)
        }
    }

    #[no_init = yes]
    fn Call(handle: u32, params: GamePtr<OsiArgumentDesc>) -> bool {
        original::Call(handle, params)
    }

    #[no_init = yes]
    fn Query(handle: u32, params: GamePtr<OsiArgumentDesc>) -> bool {
        original::Query(handle, params)
    }
}
}

unsafe fn find_osiris_globals(ctor_proc: *const u8) -> Option<OsirisStaticGlobals> {
    let addr = resolve_real_function_address(ctor_proc);

    let mut globals = [ptr::null::<()>(); 9];
    let mut found_globals = 0;

    for i in 0..0x500 {
        let ptr = addr.add(i);

        if (*ptr == 0x49 || *ptr == 0x48)
            && *ptr.add(1) == 0x8B
            && *ptr.add(3) == 0x48
            && *ptr.add(4) == 0x89
            && (*ptr.add(5) & 0xC7) == 0x05
        {
            let rel_offset = *(ptr.add(6) as *const i32) as isize;
            let osi_ptr = ptr.offset(rel_offset + 10);
            globals[found_globals] = osi_ptr as _;
            found_globals += 1;
            if found_globals == 9 {
                break;
            }
        }
    }

    if found_globals < 9 {
        err!("Could not locate global Osiris variables");
        return None;
    }

    let osiris_globals = OsirisStaticGlobals {
        variables: globals[0],
        types: globals[1],
        enums: globals[2],
        functions: GamePtr::new(globals[3] as _),
        objects: globals[4],
        goals: globals[5],
        adapters: globals[6],
        databases: globals[7],
        nodes: globals[8],
    };

    Some(osiris_globals)
}

unsafe fn resolve_real_function_address(ptr: *const u8) -> *const u8 {
    if *ptr == 0xE9 {
        let rel_offset = *(ptr.add(1) as *const i32) as isize;
        return ptr.offset(rel_offset + 5) as _;
    }

    for i in 0..64 {
        let p = ptr.add(i);
        if *p == 0x48
            && *p.add(1) == 0x83
            && *p.add(2) == 0x3D
            && *p.add(6) == 0x00
            && *p.add(13) == 0xE9
        {
            let rel_offset = *(p.add(14) as *const i32) as isize;
            return p.offset(18 + rel_offset) as _;
        }
    }

    ptr
}