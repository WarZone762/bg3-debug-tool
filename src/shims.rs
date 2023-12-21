#![allow(unused_variables, non_snake_case, dead_code)]

use std::marker::PhantomData;

use self::lua::DoConsoleCommandEvent;
use crate::console::DebugMessageType;

#[derive(Clone, Copy)]
pub struct Todo;

pub struct StaticSymbols {}

impl StaticSymbols {
    pub fn new() -> Self {
        Self {}
    }

    pub fn client_state(&self) -> Option<ecl::GameState> {
        None
    }

    pub fn server_state(&self) -> Option<esv::GameState> {
        None
    }
}

pub fn static_symbols() -> StaticSymbols {
    StaticSymbols::new()
}

pub fn luaL_loadstring(state: Todo, string: &str) -> bool {
    false
}

pub fn lua_tostring(state: Todo, arg_1: isize) -> String {
    "".into()
}

pub fn lua_pop(state: Todo, arg_1: usize) {}

pub struct LuaVirtualPin {}

impl LuaVirtualPin {
    pub fn new(state: &Todo) -> Option<Self> {
        Some(Self {})
    }

    pub fn stack(&self) -> Todo {
        Todo
    }

    pub fn state(&self) -> Todo {
        Todo
    }

    pub fn global_lifetime(&self) -> Todo {
        Todo
    }

    pub fn throw_envent(&self, arg_1: &str, params: DoConsoleCommandEvent, arg_2: bool) {}
}

pub struct EnumInfo<T> {
    t: PhantomData<T>,
}

impl<T> EnumInfo<T> {
    pub fn Find(t: T) -> Self {
        Self { t: PhantomData }
    }

    pub fn GetString(&self) -> String {
        "".into()
    }
}

pub static mut CORE_LIB_PLATFORM_INTERFACE: CoreLibPlatformInterface =
    CoreLibPlatformInterface::new();

#[derive(Clone, Copy, Debug)]
pub struct CoreLibPlatformInterface {
    pub enable_debug_break: bool,
}

impl CoreLibPlatformInterface {
    pub const fn new() -> Self {
        Self {
            enable_debug_break: false,
        }
    }
}

pub static mut EXTENDER: Extender = Extender::new();

#[derive(Clone, Copy, Debug)]
pub struct Extender {
    pub client: Client,
    pub server: Server,
    pub config: Config,
    pub debugger: Option<Debugger>,
}

impl Extender {
    pub const fn new() -> Self {
        Self {
            client: Client::new(),
            server: Server::new(),
            config: Config::new(),
            debugger: None,
        }
    }

    pub fn current_extension_state(&self) -> Option<Todo> {
        Some(Todo {})
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Client {}

impl Client {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn reset_lua_state(&self) {}

    pub fn submit_task_and_wait<T>(&self, task: impl Fn() -> T) {}
}

#[derive(Clone, Copy, Debug)]
pub struct Server {}

impl Server {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn reset_lua_state(&self) {}

    pub fn request_reset_client_lua_state(&self) -> bool {
        false
    }

    pub fn submit_task_and_wait<T>(&self, task: impl Fn() -> T) {}
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub clear_on_reset: bool,
    pub default_to_client_console: bool,
}

impl Config {
    pub const fn new() -> Self {
        Self {
            clear_on_reset: true,
            default_to_client_console: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Debugger {}

impl Debugger {
    pub fn is_ready(&self) -> bool {
        false
    }

    pub fn on_log_message(&self, r#type: DebugMessageType, msg: &str) {}
}

pub mod lua {
    use super::Todo;

    pub fn CallWithTraceback(state: Todo, arg_1: usize, arg_2: usize) -> bool {
        false
    }

    pub struct DoConsoleCommandEvent {
        pub command: String,
    }

    impl DoConsoleCommandEvent {
        pub fn new() -> Self {
            Self {
                command: String::new(),
            }
        }
    }

    pub struct StaticLifetimeStackPin {}

    impl StaticLifetimeStackPin {
        pub fn new(stack: Todo, lifetime: Todo) -> Self {
            Self {}
        }
    }
}

pub mod ecl {
    pub enum GameState {
        Menu,
        Lobby,
        Paused,
        Running,
    }
}

pub mod esv {
    pub enum GameState {
        Menu,
        Lobby,
        Paused,
        Running,
    }
}
