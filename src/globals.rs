use std::{io, net};

use crate::{binary_mappings::StaticSymbols, game_definitions::OsirisStaticGlobals};

#[macro_export]
macro_rules! info {
    ($($tt:tt)*) => {
        {
            $crate::_print!("\x1b[1m");
            $crate::_print!($($tt)*);
            $crate::_println!("\x1b[0m")
        }
    };
}

#[macro_export]
macro_rules! warn {
    ($($tt:tt)*) => {
        {
            $crate::_print!("\x1b[33m");
            $crate::_print!($($tt)*);
            $crate::_println!("\x1b[0m")
        }
    };
}

#[macro_export]
macro_rules! err {
    ($($tt:tt)*) => {
        {
            $crate::_print!("\x1b[31m");
            $crate::_print!($($tt)*);
            $crate::_println!("\x1b[0m")
        }
    };
}

#[macro_export]
macro_rules! _print {
    ($($tt:tt)*) => {
        {
            use std::io::Write;
            write!($crate::globals::Globals::io_mut(), $($tt)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! _println {
    ($($tt:tt)*) => {
        {
            use std::io::Write;
            writeln!($crate::globals::Globals::io_mut(), $($tt)*).unwrap();
        }
    };
}

static mut GLOBALS: Globals = Globals::new();

#[derive(Debug, Default)]
pub(crate) struct Globals {
    static_symbols: StaticSymbols,
    osiris_globals: Option<OsirisStaticGlobals>,
    io: Option<Io>,
}

impl Globals {
    pub const fn new() -> Self {
        Self { static_symbols: StaticSymbols::new(), osiris_globals: None, io: None }
    }

    pub fn static_symbols() -> &'static StaticSymbols {
        unsafe { &GLOBALS.static_symbols }
    }

    pub fn static_symbols_mut() -> &'static mut StaticSymbols {
        unsafe { &mut GLOBALS.static_symbols }
    }

    pub fn osiris_globals() -> &'static OsirisStaticGlobals {
        unsafe { GLOBALS.osiris_globals.as_ref().expect("osiris_globals not initialized") }
    }

    pub fn osiris_globals_mut() -> &'static mut OsirisStaticGlobals {
        unsafe { GLOBALS.osiris_globals.as_mut().expect("osiris_globals not initialized") }
    }

    pub fn osiris_globals_set(v: Option<OsirisStaticGlobals>) {
        unsafe {
            GLOBALS.osiris_globals = v;
        }
    }

    pub fn io() -> &'static Io {
        unsafe { GLOBALS.io.as_ref().expect("io not initialized") }
    }

    pub fn io_mut() -> &'static mut Io {
        unsafe { GLOBALS.io.as_mut().expect("io not initialized") }
    }

    pub fn io_set(v: Option<Io>) {
        unsafe { GLOBALS.io = v }
    }
}

#[derive(Debug)]
pub(crate) enum Io {
    StdIo(io::StdinLock<'static>, io::Stdout),
    Tcp(io::BufReader<net::TcpStream>, std::sync::Mutex<net::TcpStream>),
}

impl io::Read for Io {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Io::StdIo(stdin, _) => stdin.read(buf),
            Io::Tcp(s, _) => s.read(buf),
        }
    }
}

impl io::BufRead for Io {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            Io::StdIo(stdin, _) => stdin.fill_buf(),
            Io::Tcp(s, _) => s.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            Io::StdIo(stdin, _) => stdin.consume(amt),
            Io::Tcp(s, _) => s.consume(amt),
        }
    }
}

impl io::Write for Io {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Io::StdIo(_, stdout) => stdout.write(buf),
            Io::Tcp(_, s) => s.lock().unwrap().write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Io::StdIo(_, stdout) => stdout.flush(),
            Io::Tcp(_, s) => s.lock().unwrap().flush(),
        }
    }
}

impl Io {
    pub fn stdio() -> Self {
        Self::StdIo(io::stdin().lock(), io::stdout())
    }

    pub fn tcp(addr: impl net::ToSocketAddrs) -> Self {
        let listener = net::TcpListener::bind(addr).unwrap();
        let c = listener.accept().unwrap().0;

        Self::Tcp(io::BufReader::new(c.try_clone().unwrap()), std::sync::Mutex::new(c))
    }
}
