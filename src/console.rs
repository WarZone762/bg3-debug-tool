#![allow(dead_code)]
use crate::shims::*;
use build_time::build_time_utc;
use libc::{c_uint, wchar_t, FILE};
use std::{
    ffi::*,
    fs::File,
    io::{self, Write},
    ptr::null_mut,
    thread::JoinHandle,
};
use windows::{
    core::{s, w, PCSTR},
    Win32::{
        Globalization::{IsValidCodePage, CP_UTF8},
        System::{
            Console::{
                AllocConsole, FreeConsole, GetStdHandle, SetConsoleCP, SetConsoleCtrlHandler,
                SetConsoleMode, SetConsoleOutputCP, SetConsoleTextAttribute, SetConsoleTitleW,
                ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, FOREGROUND_BLUE,
                FOREGROUND_GREEN, FOREGROUND_INTENSITY, FOREGROUND_RED, STD_OUTPUT_HANDLE,
            },
            Diagnostics::Debug::OutputDebugStringA,
        },
    },
};

// use autocxx::prelude::*;
//
// include_cpp! {
//     #include "/hdd1/user/doc/vm/Windows 10 IoT Enterprise LTSC VM/Shared/bg3se/BG3Extender/GameDefinitions/Symbols.h"
//     safety!(unsafe)
//
//     generate!("bg3se::ecl::GameState")
//     generate!("bg3se::esv::GameState")
//
//     // concrete!("std::optional<bg3se::ecl::GameState>", O1)
//     // concrete!("std::optional<bg3se::esv::GameState>", O2)
//
//     generate!("bg3se::StaticSymbols")
// }
//
// #[no_mangle]
// unsafe extern "C" {
//     fn GetStaticSymbols() -> &'static mut ffi::bg3se::StaticSymbols;
// }

// #[cxx::bridge(namespace = "bg3se")]
// mod ffi {
//     unsafe extern "C++" {
//         include!("/hdd1/user/doc/vm/Windows 10 IoT Enterprise LTSC VM/Shared/bg3se/BG3Extender/GameDefinitions/Symbols.h");
//
//         type StaticSymbols;
//         fn GetClientState(self: &StaticSymbols) -> Foo;
//         fn GetServerState(self: &StaticSymbols) -> Foo;
//         fn GetStaticSymbols() -> &'static StaticSymbols;
//     }
// }

extern "C" {
    pub fn freopen_s(
        stream: *mut *mut FILE,
        file_name: *const c_char,
        mode: *const c_char,
        old_stream: *mut FILE,
    ) -> c_int;

    pub fn __acrt_iob_func(_lx: c_uint) -> *mut FILE;
}

pub fn stdin() -> *mut FILE {
    unsafe { __acrt_iob_func(0) }
}

pub fn stdout() -> *mut FILE {
    unsafe { __acrt_iob_func(1) }
}

pub fn stderr() -> *mut FILE {
    unsafe { __acrt_iob_func(2) }
}

#[derive(Debug)]
pub enum Error {
    Windows(windows::core::Error),
    Io(io::Error),
    Utf8(std::str::Utf8Error),
    Custom(String),
}

impl From<windows::core::Error> for Error {
    fn from(value: windows::core::Error) -> Self {
        Error::Windows(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Error::Utf8(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[macro_export]
macro_rules! _debug {
    ($type:expr, $($args:expr),*) => {
        unsafe {
            if let Some(x) = &mut CONSOLE {
                x.print($type, &format!($($args),*)).unwrap();
            }
        }
    };
}

#[macro_export]
macro_rules! DEBUG {
    ($($args:expr),*) => {
        _debug!(DebugMessageType::Debug, $($args),*)
    };
}

#[macro_export]
macro_rules! INFO {
    ($($args:expr),*) => {
        _debug!(DebugMessageType::Info, $($args),*)
    };
}

#[macro_export]
macro_rules! WARN {
    ($($args:expr),*) => {
        _debug!(DebugMessageType::Warning, $($args),*)
    };
}

#[macro_export]
macro_rules! ERR {
    ($($args:expr),*) => {
        _debug!(DebugMessageType::Err, $($args),*)
    };
}

const CURRENT_VERSION: u32 = 12;

static mut CONSOLE: Option<DebugConsole> = None;

pub struct DebugConsole {
    pub console: Console,
    console_running: bool,
    server_context: bool,
    multi_line_mode: bool,
    thread: Option<JoinHandle<()>>,
    multi_line_command: String,
}

impl DebugConsole {
    pub fn new() -> Result<Self> {
        let mut c = Self {
            console: Console::new()?,
            console_running: true,
            server_context: unsafe { !EXTENDER.config.default_to_client_console },
            multi_line_mode: false,
            thread: None,
            multi_line_command: String::new(),
        };
        c.console.enabled = true;

        unsafe { SetConsoleTitleW(w!("BG3 Script Extender Debug Console"))? };

        macro_rules! DEBUG {
            ($($args:expr),*) => {
                c.print(DebugMessageType::Debug, &format!($($args),*))?;
            };
        }

        DEBUG!("******************************************************************************");
        DEBUG!("*                                                                            *");
        DEBUG!("*                     BG3 Script Extender Debug Console                      *");
        DEBUG!("*                                                                            *");
        DEBUG!("******************************************************************************");
        DEBUG!("");
        DEBUG!("BG3Ext v{CURRENT_VERSION} built on {}", build_time_utc!());

        c.thread = Some(std::thread::spawn(|| unsafe {
            if let Some(c) = &mut CONSOLE {
                c.console_thread()
            }
        }));

        Ok(c)
    }

    pub fn set_color(&mut self, r#type: DebugMessageType) -> Result<()> {
        self.console.set_color(r#type)
    }

    pub fn print(&mut self, r#type: DebugMessageType, msg: &str) -> Result<()> {
        self.console.print(r#type, msg)?;

        #[cfg(not(feature = "osi-no-debugger"))]
        if self.console.silence
            && let Some(debugger) = unsafe { EXTENDER.debugger }
            && debugger.is_ready()
        {
            debugger.on_log_message(r#type, msg);
        }

        Ok(())
    }

    pub fn handle_command(&mut self, cmd: &str) {
        match cmd {
            "" => (),
            "server" => {
                DEBUG!("Switching to server context.");
                self.server_context = true;
            }
            "cilent" => {
                DEBUG!("Switching to client context.");
                self.server_context = false;
            }
            #[cfg(feature = "debug")]
            "debugbreak" => {
                unsafe {
                    CORE_LIB_PLATFORM_INTERFACE.enable_debug_break =
                        !CORE_LIB_PLATFORM_INTERFACE.enable_debug_break
                };
                if unsafe { CORE_LIB_PLATFORM_INTERFACE.enable_debug_break } {
                    DEBUG!("Debug breaks ON");
                } else {
                    DEBUG!("Debug breaks OFF");
                }
            }
            "reset" => self.reset_lua(),
            "reset server" => self.reset_lua_server(),
            "reset client" => self.reset_lua_client(),
            "silence on" => {
                DEBUG!("Silent mode ON");
                self.console.silence = true;
            }
            "silence off" => {
                DEBUG!("Silent mode OFF");
                self.console.silence = false;
            }
            "clear" => self.clear(),
            "help" => self.print_help(),
            _ => self.exec_lua_command(cmd),
        }
    }

    pub fn clear(&mut self) {
        self.console.clear()
    }

    pub fn open_log_file(&mut self, path: &str) -> Result<()> {
        self.console.open_log_file(path)
    }

    pub fn close_log_file(&mut self) {
        self.console.close_log_file()
    }

    pub fn set_log_callback(&mut self, callback: impl Fn(&str) + 'static) {
        self.console.set_log_callback(callback)
    }

    fn console_thread(&mut self) {
        let mut buf = String::new();
        while self.console_running {
            io::stdin().read_line(&mut buf).unwrap();
            buf.clear();

            DEBUG!("Entering server Lua console,");

            while self.console_running {
                self.console.input_enabled = true;
                if self.server_context {
                    print!("S");
                } else {
                    print!("C");
                }

                if self.multi_line_mode {
                    print!(" -->> ");
                } else {
                    print!(" >> ");
                }

                io::stdout().flush().unwrap();
                buf.clear();
                io::stdin().read_line(&mut buf).unwrap();
                buf.truncate(buf.trim_end().len());
                self.console.input_enabled = false;

                if !self.multi_line_mode {
                    if buf == "exit" {
                        break;
                    } else if buf == "--[[" {
                        self.multi_line_mode = true;
                        self.multi_line_command.clear();
                        continue;
                    }

                    self.handle_command(&buf);
                } else if buf == "]]--" {
                    self.multi_line_mode = false;
                    self.handle_command(&self.multi_line_command.clone());
                } else {
                    self.multi_line_command.push_str(&buf);
                    self.multi_line_command.push('\n');
                }
            }

            DEBUG!("Exiting console mode.");
        }
    }

    fn submit_task_and_wait<T>(&self, server: bool, task: impl Fn() -> T) {
        // let state = unsafe { GetStaticSymbols() };
        // let client_state = state.GetClientState();
        // let server_state = state.GetServerState();
        //
        // println!("{client_state:#X}, {server_state:#X}");

        if server {
            match static_symbols().server_state() {
                None => ERR!(
                    "Cannot queue server commands when the server state machine is not initialized"
                ),
                Some(esv::GameState::Paused) | Some(esv::GameState::Running) => unsafe {
                    EXTENDER.server.submit_task_and_wait(task)
                },
                Some(state) => ERR!(
                    "Cannot queue server commands in game state {}",
                    EnumInfo::<esv::GameState>::Find(state).GetString()
                ),
            }
        } else {
            match static_symbols().client_state() {
                None => ERR!(
                    "Cannot queue client commands when the client state machine is not initialized"
                ),
                Some(ecl::GameState::Menu)
                | Some(ecl::GameState::Lobby)
                | Some(ecl::GameState::Paused)
                | Some(ecl::GameState::Running) => unsafe {
                    EXTENDER.client.submit_task_and_wait(task)
                },
                Some(state) => ERR!(
                    "Cannot queue client commands in game state {}",
                    EnumInfo::<ecl::GameState>::Find(state).GetString()
                ),
            }
        }
    }

    fn print_help(&self) {
        DEBUG!(
            "Anything typed in will be executed as Lua code except the following special commands:"
        );
        DEBUG!("  server - Switch to server context");
        DEBUG!("  client - Switch to client context");
        DEBUG!("  reset client - Reset client Lua state");
        DEBUG!("  reset server - Reset server Lua state");
        DEBUG!("  reset - Reset client and server Lua states");
        DEBUG!("  silence <on|off> - Enable/disable silent mode (log output when in input mode)");
        DEBUG!("  clear - Clear the console");
        DEBUG!("  exit - Leave console mode");
        DEBUG!("  !<cmd> <arg1> ... <argN> - Trigger Lua \"ConsoleCommand\" event with arguments cmd, arg1, ..., argN");
    }

    fn reset_lua(&mut self) {
        self.clear_from_reset();

        DEBUG!("Resetting Lua states.");
        self.submit_task_and_wait(true, || unsafe { EXTENDER.server.reset_lua_state() });

        self.submit_task_and_wait(false, || unsafe {
            if !EXTENDER.server.request_reset_client_lua_state() {
                EXTENDER.client.reset_lua_state();
            }
        })
    }

    fn reset_lua_client(&mut self) {
        self.clear_from_reset();

        DEBUG!("Resetting client Lua state,");
        self.submit_task_and_wait(false, || unsafe {
            if !EXTENDER.server.request_reset_client_lua_state() {
                EXTENDER.client.reset_lua_state();
            }
        })
    }

    fn reset_lua_server(&mut self) {
        self.clear_from_reset();

        DEBUG!("Resetting server Lua state.");
        self.submit_task_and_wait(true, || unsafe { EXTENDER.server.reset_lua_state() });
    }

    fn exec_lua_command(&mut self, cmd: &str) {
        self.submit_task_and_wait(self.server_context, || {
            let Some(state) = (unsafe { EXTENDER.current_extension_state() }) else {
                ERR!("Extension not initialized!");
                return None;
            };

            let Some(pin) = LuaVirtualPin::new(&state) else {
                ERR!("Lua state not initialized!");
                return None;
            };

            if let Some(cmd) = cmd.strip_prefix('!') {
                let mut params = lua::DoConsoleCommandEvent::new();
                params.command = cmd.into();
                pin.throw_envent("DoConsoleCommand", params, false);
            } else {
                lua::StaticLifetimeStackPin::new(pin.stack(), pin.global_lifetime());
                let l = pin.state();
                if luaL_loadstring(l, cmd) || lua::CallWithTraceback(l, 0, 0) {
                    ERR!("{}", lua_tostring(l, -1));
                    lua_pop(l, 1);
                }
            }

            Some(())
        });
    }

    fn clear_from_reset(&mut self) {
        // Clear console if the setting is enabled
        if unsafe { EXTENDER.config.clear_on_reset } {
            self.clear();
        }
    }
}

#[derive(Default)]
pub struct Console {
    pub enabled: bool,
    input_enabled: bool,
    silence: bool,
    log_file: Option<File>,
    log_callback: Option<Box<dyn Fn(&str)>>,
}

impl Console {
    pub fn new() -> Result<Self> {
        unsafe {
            AllocConsole()?;
            if IsValidCodePage(CP_UTF8).into() {
                SetConsoleCP(CP_UTF8)?;
                SetConsoleOutputCP(CP_UTF8)?;
            }

            let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE)?;
            // Disabe <C-C> handling
            SetConsoleMode(
                stdout_handle,
                ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
            )?;
            SetConsoleCtrlHandler(None, true)?;

            let mut input_stream: *mut FILE = null_mut();
            let mut output_stream: *mut FILE = null_mut();
            freopen_s(
                &mut input_stream,
                s!("CONIN$").as_ptr() as _,
                s!("r").as_ptr() as _,
                stdin(),
            );
            freopen_s(
                &mut output_stream,
                s!("CONOUT$").as_ptr() as _,
                s!("w").as_ptr() as _,
                stdout(),
            );
        }

        Ok(Self::default())
    }

    pub fn set_color(&mut self, r#type: DebugMessageType) -> Result<()> {
        let attributes = match r#type {
            DebugMessageType::Debug => FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE,
            DebugMessageType::Info => {
                FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE | FOREGROUND_INTENSITY
            }
            DebugMessageType::Osiris => FOREGROUND_INTENSITY | FOREGROUND_GREEN | FOREGROUND_BLUE,
            DebugMessageType::Warning => FOREGROUND_RED | FOREGROUND_GREEN,
            DebugMessageType::Err => FOREGROUND_RED | FOREGROUND_INTENSITY,
        };

        unsafe { SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE)?, attributes)? };

        Ok(())
    }

    pub fn print(&mut self, r#type: DebugMessageType, msg: &str) -> Result<()> {
        if self.enabled && (!self.input_enabled || !self.silence) {
            unsafe {
                self.set_color(r#type)?;
                let c_msg = CString::new(msg).unwrap();
                OutputDebugStringA(PCSTR::from_raw(c_msg.as_ptr() as _));
                OutputDebugStringA(PCSTR::from_raw(b"\r\n" as _));
                println!("{msg}");
                io::stdout().flush()?;
                self.set_color(DebugMessageType::Debug)?;
            }
        }

        if let Some(cb) = &self.log_callback {
            cb(msg);
        }

        if let Some(log_file) = &mut self.log_file {
            log_file.write_all(msg.as_bytes())?;
            log_file.write_all(b"\r\n")?;
            log_file.flush()?;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        // Clear screen, move cursor to top-left and clear scrollback
        print!("\x1b[2J\x1b[H\x1b[3J");
    }

    pub fn open_log_file(&mut self, path: &str) -> Result<()> {
        if self.log_file.is_some() {
            self.close_log_file();
        }

        match File::options().append(true).open(path) {
            Ok(log_file) => {
                self.log_file = Some(log_file);
                Ok(())
            }
            Err(err) => {
                ERR!("Failed to open log file '{path}'");
                Err(err.into())
            }
        }
    }

    pub fn close_log_file(&mut self) {
        self.log_file.take();
    }

    pub fn set_log_callback(&mut self, callback: impl Fn(&str) + 'static) {
        self.log_callback = Some(Box::new(callback));
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        unsafe { FreeConsole().unwrap() };
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DebugMessageType {
    Debug,
    Info,
    Osiris,
    Warning,
    Err,
}

impl TryFrom<u32> for DebugMessageType {
    type Error = String;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Debug),
            1 => Ok(Self::Info),
            2 => Ok(Self::Osiris),
            3 => Ok(Self::Warning),
            4 => Ok(Self::Err),
            _ => Err(format!("Unknown DebugMessageType({value})")),
        }
    }
}

#[no_mangle]
unsafe extern "C" fn ConsoleNew() -> usize {
    1
}

#[no_mangle]
unsafe extern "C" fn ConsoleCreate(_id: usize) {
    CONSOLE = Some(DebugConsole::new().unwrap());
}

#[no_mangle]
unsafe extern "C" fn ConsoleDestroy(_id: usize) {
    CONSOLE.take();
}

#[no_mangle]
unsafe extern "C" fn ConsoleOpenLogFile(_id: usize, path: *const wchar_t) {
    if let Some(x) = &mut CONSOLE {
        x.open_log_file(&widestring::U16CStr::from_ptr_str(path).to_string().unwrap())
            .unwrap()
    }
}

#[no_mangle]
unsafe extern "C" fn ConsoleCloseLogFile(_id: usize) {
    if let Some(x) = &mut CONSOLE {
        x.close_log_file()
    }
}

#[no_mangle]
unsafe extern "C" fn ConsolePrint(_id: usize, r#type: u32, msg: *const c_char) {
    if let Some(x) = &mut CONSOLE {
        x.print(
            r#type.try_into().unwrap(),
            CStr::from_ptr(msg).to_str().unwrap(),
        )
        .unwrap()
    }
}

#[no_mangle]
unsafe extern "C" fn ConsoleSetColor(_id: usize, r#type: u32) {
    if let Some(x) = &mut CONSOLE {
        x.set_color(r#type.try_into().unwrap()).unwrap()
    }
}

#[no_mangle]
unsafe extern "C" fn ConsoleClear(_id: usize) {
    if let Some(x) = &mut CONSOLE {
        x.clear()
    }
}

#[no_mangle]
unsafe extern "C" fn ConsoleEnableOutput(_id: usize, enabled: bool) {
    if let Some(x) = &mut CONSOLE {
        x.console.enabled = enabled
    }
}

#[no_mangle]
unsafe extern "C" fn ConsoleSetLogCallback(_id: usize, callback: extern "C" fn(*const u8)) {
    if let Some(x) = &mut CONSOLE {
        x.set_log_callback(move |msg| callback(msg.as_ptr() as _))
    }
}

// #[cfg(test)]
// mod tests {
//     #![allow(unused_imports)]
//     use super::*;
//
//     #[test]
//     fn it_works() {
//         assert_eq!(2, 2);
//     }
// }
