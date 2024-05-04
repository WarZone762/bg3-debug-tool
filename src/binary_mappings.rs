#![allow(dead_code, unused_variables)]
use std::{fmt::Debug, mem::size_of_val};

use anyhow::bail;
use widestring::U16CString;
use windows::{
    core::{w, PCWSTR},
    Win32::System::{
        Diagnostics::Debug::{ImageNtHeader, IMAGE_SECTION_HEADER},
        LibraryLoader::{GetModuleHandleW, LoadLibraryW},
        ProcessStatus::GetModuleInformation,
        Threading::GetCurrentProcess,
    },
};

use self::mappings::{
    BinaryMappings, Condition, ConditionValue, Mapping, MappingOrDllImport, TargetType,
    TargetValue, TargetsOrPatch,
};
use crate::{
    err,
    game_definitions::{
        FixedString, GamePtr, GlobalTemplateManager, LSStringView, PassivePrototypeManager,
        ResourceManager, SpellPrototypeManager, StatusPrototypeManager, TextureAtlasMap,
        TranslatedStringRepository,
    },
    globals::Globals,
    warn,
};

const BINARY_MAPPINGS_XML: &str = include_str!("BinaryMappings.xml");

pub(crate) fn init_static_symbols() -> anyhow::Result<()> {
    let binary_mappings: xml::BinaryMappings = quick_xml::de::from_str(BINARY_MAPPINGS_XML)?;
    let mut symbol_mapper = SymbolMapper::new()?;

    symbol_mapper.populate_mappings(binary_mappings.try_into().unwrap());

    *Globals::static_symbols_mut() = symbol_mapper.static_symbols;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SymbolMapper {
    pub main_module: ModuleInfo,
    pub static_symbols: StaticSymbols,
}

impl SymbolMapper {
    pub fn new() -> windows::core::Result<Self> {
        let main_module =
            if unsafe { GetModuleHandleW(w!("bg3.exe")) }.is_ok_and(|x| !x.is_invalid()) {
                ModuleInfo::load("bg3.exe")
            } else {
                ModuleInfo::load("bg3_dx11.exe")
            }?;

        Ok(Self { main_module, static_symbols: StaticSymbols::default() })
    }

    pub fn populate_mappings(&mut self, binary_mappings: BinaryMappings) {
        for mapping in binary_mappings.data {
            match mapping {
                MappingOrDllImport::DllImport(_) => (),
                MappingOrDllImport::Mapping(mapping) => self.add_mapping(&mapping),
            }
        }
    }

    fn add_mapping(&mut self, mapping: &Mapping) {
        mapping.pattern.scan(
            unsafe {
                std::slice::from_raw_parts(self.main_module.text_start, self.main_module.text_size)
            },
            |addr| {
                match &mapping.targets_or_patch {
                    TargetsOrPatch::Targets(targets) => {
                        for t in targets {
                            match &t.value {
                                TargetValue::Symbol(s) => match t.r#type {
                                    TargetType::Absolute => (),
                                    TargetType::Indirect => {
                                        if let Some(addr) = unsafe {
                                            asm_resolve_instruction_ref(
                                                addr.as_ptr().offset(t.offset),
                                            )
                                        } {
                                            if let Err(x) =
                                                self.static_symbols.set(s.as_str(), addr)
                                            {
                                                warn!("{x}");
                                            }
                                        }
                                    }
                                },
                                TargetValue::NextSymbol { value, offset } => (),
                                TargetValue::EngineCallback(_) => (),
                            }
                        }
                    }
                    TargetsOrPatch::Patch(_) => (),
                }
                if let Some(cond) = &mapping.condition {
                    Self::evaluate_symbol_condition(cond, addr.as_ptr());
                }
                Some(())
            },
        );
    }

    fn evaluate_symbol_condition(cond: &Condition, ptr: *const u8) {
        match &cond.value {
            ConditionValue::String(str) => unsafe {
                if let Some(tgt_str) = asm_resolve_instruction_ref(ptr.offset(cond.offset)) {
                    let cstr = std::ffi::CStr::from_ptr(tgt_str as _);

                    if cstr.to_str() != Ok(str) {
                        // println!("{cstr:?} != \"{str}\"");
                    }
                }
            },
            ConditionValue::FixedString(str) => (),
            ConditionValue::FixedStringIndirect(str) => (),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleInfo {
    pub start: *const u8,
    pub size: usize,
    pub text_start: *const u8,
    pub text_size: usize,
}

impl ModuleInfo {
    pub fn load(name: &str) -> windows::core::Result<Self> {
        unsafe {
            let lib = LoadLibraryW(PCWSTR(U16CString::from_str_truncate(name).as_ptr()))?;
            let mut module_info = Default::default();
            GetModuleInformation(
                GetCurrentProcess(),
                lib,
                &mut module_info,
                size_of_val(&module_info) as _,
            )?;

            let start = module_info.lpBaseOfDll as *const u8;
            let size = module_info.SizeOfImage as usize;

            let mut text_start = start;
            let mut text_size = size;

            let nt_header = ImageNtHeader(start as _);
            let section_header = nt_header.add(1) as *const IMAGE_SECTION_HEADER;

            for i in 0..(*nt_header).FileHeader.NumberOfSections {
                if &(*section_header.add(i as usize)).Name[..5] == b".text" {
                    text_start = start.add((*section_header).VirtualAddress as _);
                    text_size = (*section_header).SizeOfRawData as _;
                }
            }

            Ok(Self { start, size, text_start, text_size })
        }
    }
}

macro_rules! static_symbols {
    {$($name: ident: $type: ty,)*} => {
        #[allow(non_snake_case, dead_code)]
        #[derive(Clone, Copy, Debug, Default)]
        pub(crate) struct StaticSymbols {
            $(
                pub $name: Option<$type>,
            )*
        }

        unsafe impl Send for StaticSymbols {}

        impl StaticSymbols {
            pub const fn new() -> Self {
                Self {
                    $(
                        $name: None,
                    )*
                }
            }

            pub fn set(&mut self, name: &str, value: *const u8) -> anyhow::Result<()> {
                let symbol_name = StaticSymbolName::from_str(name)?;

                match symbol_name {
                    $(
                        StaticSymbolName::$name => {
                            if self.$name.is_some() {
                                bail!("mapping '{name}' is already bound");
                            }
                            self.$name = Some(unsafe { std::mem::transmute(value) })
                        }
                    )*
                }

                Ok(())
            }

            #[allow(dead_code)]
            pub fn create_hash_map<T: Default>() -> std::collections::HashMap<&'static str, T> {
                std::collections::HashMap::from([
                    $(
                        (stringify!($name), Default::default()),
                    )*
                ])
            }
        }

        #[allow(non_camel_case_types, dead_code)]
        #[derive(Clone, Copy, Debug)]
        pub(crate) enum StaticSymbolName {
            $(
                $name,
            )*
        }

        impl StaticSymbolName {
            pub fn from_str(k: &str) -> anyhow::Result<Self> {
                match k {
                    $(
                        stringify!($name) => Ok(Self::$name),
                    )*
                    x => bail!("unknown static symbol '{x}'"),
                }
            }
        }
    };
}

static_symbols! {
    ls__FixedString__GetString: extern "C" fn(GamePtr<FixedString>, GamePtr<LSStringView>) -> GamePtr<LSStringView>,
    ls__FixedString__IncRef: fn(),
    ls__GlobalStringTable__MainTable__CreateFromString: fn(),
    ls__GlobalStringTable__MainTable__DecRef: fn(),
    ls__gGlobalStringTable: *const (),

    ls__FileReader__ctor: fn(),
    ls__FileReader__dtor: fn(),
    ls__PathRoots: *const (),
    App__Ctor: extern "C" fn(*const ()) -> *const (),
    App__UpdatePaths: fn(),

    ecl__EoCClient: *const (),
    esv__EoCServer: *const (),

    ecl__EoCClient__HandleError: fn(),

    ls__gTranslatedStringRepository: GamePtr<GamePtr<TranslatedStringRepository<'static>>>,

    ecl__gGameStateEventManager: *const (),
    esv__gGameStateEventManager: *const (),
    ecl__GameStateThreaded__GameStateWorker__DoWork: fn(),
    esv__GameStateThreaded__GameStateWorker__DoWork: fn(),
    ecl__GameStateMachine__Update: extern "C" fn(*const (), *const ()),
    esv__GameStateMachine__Update: fn(),
    App__LoadGraphicSettings: fn(),

    ecs__EntityWorld__Update: fn(),

    eoc__SpellPrototypeManager: GamePtr<GamePtr<SpellPrototypeManager>>,
    eoc__SpellPrototype__Init: fn(),

    eoc__StatusPrototypeManager: GamePtr<GamePtr<StatusPrototypeManager>>,
    eoc__StatusPrototype__Init: fn(),

    eoc__PassiveManager: *const (),
    eoc__Passive__Init: fn(),

    esv__OsirisVariableHelper__SavegameVisit: fn(),

    esv__StatusMachine__CreateStatus: fn(),
    esv__StatusMachine__ApplyStatus: fn(),

    stats__DealDamageFunctor__ApplyDamage: fn(),
    esv__StatsSystem__ThrowDamageEvent: fn(),

    stats__Functors__ExecuteType1: fn(),
    stats__Functors__ExecuteType2: fn(),
    stats__Functors__ExecuteType3: fn(),
    stats__Functors__ExecuteType4: fn(),
    stats__Functors__ExecuteType5: fn(),
    stats__Functors__ExecuteType6: fn(),
    stats__Functors__ExecuteType7: fn(),
    stats__Functors__ExecuteType8: fn(),

    gRPGStats: *const (),
    RPGStats__Load: fn(),
    RPGStats__PreParseDataFolder: fn(),
    stats__Object__SetPropertyString: fn(),

    esv__LevelManager: *const (),
    ls__GlobalTemplateManager: GamePtr<GamePtr<GlobalTemplateManager>>,
    esv__CacheTemplateManager: *const (),

    esv__SavegameManager: *const (),

    AppInstance: *const (),

    Libraries: *const (),

    ls__gGlobalAllocator: *const (),
    ls__GlobalAllocator__Alloc: fn(),
    ls__GlobalAllocator__Free: fn(),

    eoc__gGuidResourceManager: *const (),
    ls__gGlobalResourceManager: GamePtr<GamePtr<ResourceManager>>,

    ls__VirtualTextureResource__Load: fn(),
    ls__VirtualTextureResource__Unload: fn(),
    ls__VirtualTextureResource__Transcode: fn(),

    ls__GlobalSwitches: *const (),

    Kernel_FindFirstFileW: fn(),
    Kernel_FindNextFileW: fn(),
    Kernel_FindClose: fn(),

    eoc__InterruptPrototypeManager: *const(),
    eoc__PassivePrototype__Init: fn(),
    eoc__PassivePrototypeManager: GamePtr<GamePtr<PassivePrototypeManager>>,
    eoc__InterruptPrototype__Init: fn(),

    Noesis__SymbolManager__Buf1: *const(),
    Noesis__gReflection: *const(),
    Noesis__GUI__LoadXaml: fn(),
    Noesis__Visual__RemoveVisualChild: fn(),
    Noesis__Visual__AddVisualChild: fn(),
    ls__UIStateMachine__FireStateEvent2: *const(),

    ls__gTextureAtlasMap: GamePtr<GamePtr<TextureAtlasMap>>,
}

unsafe fn asm_resolve_instruction_ref(insn: *const u8) -> Option<*const u8> {
    Some(match (*insn, *(insn.add(1)), *(insn.add(2))) {
        // Call (4b operand) instruction
        (0xE8 | 0xE9, ..) => {
            let rel = (insn.add(1) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 5)
        }
        // MOV to 32-bit register (4b operand) instruction
        (0x8B, x, _) if x < 0x20 => {
            let rel = (insn.add(2) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 6)
        }
        // MOV/LEA (4b operand) instruction
        (0x44 | 0x48 | 0x4C, 0x8D | 0x8B | 0x89, _) => {
            let rel = (insn.add(3) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 7)
        }
        // MOVSXD (4b operand) instruction
        (0x48, 0x63, _) => {
            let rel = (insn.add(3) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 7)
        }
        // MOVZX (4b operand) instruction
        (0x44, 0x0F, 0xB7) => {
            let rel = (insn.add(4) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 8)
        }
        // MOVZX (4b operand) instruction
        (0x0F, 0xB7, _) => {
            let rel = (insn.add(3) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 7)
        }
        // CMP reg, [rip+xx] (4b operand) instruction
        (0x48, 0x3B, x) if x & 0x0F == 0x0D => {
            let rel = (insn.add(3) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 7)
        }
        // MOV cs:xxx, <imm4> instruction
        (0xC7, 0x05, _) => {
            let rel = (insn.add(2) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 10)
        }
        // OR ax, word ptr [cs:<imm4>] intruction
        (0x66, 0x0B, _) => {
            let rel = (insn.add(3) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 7)
        }
        // CMP, reg, [rip+xx] intruction
        (0x3B, ..) => {
            let rel = (insn.add(2) as *const i32).read_unaligned() as isize;
            insn.offset(rel + 6)
        }
        _ => {
            err!(
                "asm_resolve_instruction_ref(): Not a supported CALL, MOV, LEA or CMP instruction \
                 at {insn:#?}"
            );
            return None;
        }
    })
}

pub(crate) mod mappings {
    #![allow(dead_code)]
    use std::fmt::Debug;

    use anyhow::{anyhow, bail};

    use super::{xml, StaticSymbolName};
    use crate::warn;

    #[derive(Clone, Debug)]
    pub(crate) struct BinaryMappings {
        pub version: String,
        pub default: bool,
        pub data: Vec<MappingOrDllImport>,
    }

    impl TryFrom<xml::BinaryMappings> for BinaryMappings {
        type Error = anyhow::Error;

        fn try_from(value: xml::BinaryMappings) -> Result<Self, Self::Error> {
            Ok(Self {
                version: value.mappings.version,
                default: value.mappings.default,
                data: value
                    .mappings
                    .inner
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            })
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) enum MappingOrDllImport {
        DllImport(DllImport),
        Mapping(Mapping),
    }

    impl TryFrom<xml::MappingOrDllImport> for MappingOrDllImport {
        type Error = anyhow::Error;

        fn try_from(value: xml::MappingOrDllImport) -> Result<Self, Self::Error> {
            Ok(match value {
                xml::MappingOrDllImport::DllImport(x) => Self::DllImport(x.try_into()?),
                xml::MappingOrDllImport::Mapping(x) => Self::Mapping(x.try_into()?),
            })
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct DllImport {
        pub module: String,
        pub proc: String,
        pub symbol: StaticSymbolName,
    }

    impl TryFrom<xml::DllImport> for DllImport {
        type Error = anyhow::Error;

        fn try_from(value: xml::DllImport) -> Result<Self, Self::Error> {
            Ok(Self {
                module: value.module,
                proc: value.proc,
                symbol: StaticSymbolName::from_str(&value.symbol)?,
            })
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Mapping {
        pub name: String,
        pub critical: bool,
        pub allow_fail: bool,
        pub scope: MappingScope,
        pub pattern: Pattern,
        pub condition: Option<Condition>,
        pub targets_or_patch: TargetsOrPatch,
    }

    impl TryFrom<xml::Mapping> for Mapping {
        type Error = anyhow::Error;

        fn try_from(value: xml::Mapping) -> Result<Self, Self::Error> {
            let mut condition = None;
            let mut pattern = None;
            let mut targets = Vec::new();
            let mut patch = None;

            for prop in value.props {
                match prop {
                    xml::MappingProperty::Patch(x) => patch = Some(x),
                    xml::MappingProperty::Target(x) => targets.push(x),
                    xml::MappingProperty::Condition(x) => condition = Some(x),
                    xml::MappingProperty::Pattern(x) => pattern = Some(x),
                }
            }

            let pattern =
                pattern.ok_or_else(|| anyhow!("no Pattern for {}", value.name))?.try_into()?;

            let targets_or_patch = if !targets.is_empty() && patch.is_none() {
                TargetsOrPatch::Targets(
                    targets
                        .into_iter()
                        .map(|x| Target::from_parsed(x, &pattern))
                        .collect::<Result<_, _>>()?,
                )
            } else if targets.is_empty()
                && let Some(patch) = patch
            {
                TargetsOrPatch::Patch(Patch::from_parsed(patch, &pattern)?)
            } else {
                bail!("both Targets and Patch defined for {}", value.name);
            };

            Ok(Self {
                name: value.name,
                critical: value.critical,
                allow_fail: value.allow_fail,
                scope: value.scope.try_into()?,
                condition: condition.map(|x| Condition::from_parsed(x, &pattern)).transpose()?,
                pattern,
                targets_or_patch,
            })
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) enum MappingScope {
        Text,
        Custom,
    }

    impl TryFrom<xml::MappingScope> for MappingScope {
        type Error = anyhow::Error;

        fn try_from(value: xml::MappingScope) -> Result<Self, Self::Error> {
            match value {
                xml::MappingScope::Text => Ok(Self::Text),
                xml::MappingScope::Custom => Ok(Self::Custom),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Pattern {
        bytes: Vec<PatternByte>,
        anchors: Vec<(String, usize)>,
    }

    impl TryFrom<&str> for Pattern {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            let mut bytes = Vec::new();
            let mut anchors = Vec::new();

            let mut chars = value.chars();
            while let Some(char) = chars.next() {
                match char {
                    ' ' | '\t' | '\r' | '\n' => continue,
                    '/' => {
                        while let Some(c) = chars.next()
                            && c != '\r'
                            && c != '\n'
                        {}
                    }
                    '@' => {
                        let mut name = String::with_capacity(4);
                        name.push('@');
                        while let Some(c) = chars.next()
                            && c.is_alphanumeric()
                        {
                            name.push(c);
                        }

                        if name.is_empty() {
                            bail!("empty anchor name found");
                        }

                        anchors.push((name, bytes.len()));
                    }
                    // TODO: make sane
                    c1 => match (c1, chars.next(), chars.next()) {
                        (_, None, _) => {
                            bail!("bytes must be 2 characters long");
                        }
                        (_, _, Some(x)) if x.is_alphanumeric() => {
                            bail!("bytes must be separated by whitespace");
                        }
                        ('?', Some('?'), _) => bytes.push(PatternByte::new(0, 0)),
                        (_, Some(c2), _) => bytes.push(PatternByte::new(
                            hex_to_u8(c1, c2)
                                .ok_or_else(|| anyhow!("invalid hex digit: {c1}{c2}"))?,
                            0xFF,
                        )),
                    },
                }
            }

            match bytes.first() {
                None => bail!("zero-length patterns not allowed"),
                Some(first) if first.mask != 0xFF => {
                    bail!("first byte of pattern must be an exact match")
                }
                Some(_) => Ok(Pattern { bytes, anchors }),
            }
        }
    }

    impl TryFrom<&String> for Pattern {
        type Error = anyhow::Error;

        fn try_from(value: &String) -> Result<Self, Self::Error> {
            Self::try_from(value.as_str())
        }
    }

    impl TryFrom<String> for Pattern {
        type Error = anyhow::Error;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            Self::try_from(value.as_str())
        }
    }

    impl Pattern {
        pub fn match_pattern(&self, mem: &[u8]) -> bool {
            for (x, byte) in self.bytes.iter().zip(mem) {
                if byte & x.mask != x.value {
                    return false;
                }
            }

            true
        }

        pub fn scan_prefix_1(&self, mem: &[u8], mut cb: impl FnMut(&[u8]) -> Option<()>) {
            let initial = self.bytes[0].value;

            for win in mem
                .windows(self.bytes.len())
                .filter(|win| win[0] == initial && self.match_pattern(win))
            {
                if cb(win).is_some() {
                    return;
                }
            }

            warn!("unable to find {self:?}");
        }

        pub fn scan_prefix_2(&self, mem: &[u8], mut cb: impl FnMut(&[u8]) -> Option<()>) {
            let initial = u16::from_ne_bytes([self.bytes[0].value, self.bytes[1].value]);

            for win in mem.windows(self.bytes.len()).filter(|win| {
                u16::from_ne_bytes([win[0], win[1]]) == initial && self.match_pattern(win)
            }) {
                if cb(win).is_some() {
                    return;
                }
            }

            warn!("unable to find {self:?}");
        }

        pub fn scan_prefix_4(&self, mem: &[u8], mut cb: impl FnMut(&[u8]) -> Option<()>) {
            let initial = u32::from_ne_bytes([
                self.bytes[0].value,
                self.bytes[1].value,
                self.bytes[2].value,
                self.bytes[3].value,
            ]);

            for win in mem.windows(self.bytes.len()).filter(|win| {
                u32::from_ne_bytes([win[0], win[1], win[2], win[3]]) == initial
                    && self.match_pattern(win)
            }) {
                if cb(win).is_some() {
                    return;
                }
            }

            warn!("unable to find {self:?}");
        }

        pub fn scan(&self, mem: &[u8], cb: impl FnMut(&[u8]) -> Option<()>) {
            let prefix_len = self.bytes.iter().position(|b| b.mask != 0xFF).unwrap_or(0);

            match prefix_len {
                4.. => self.scan_prefix_4(mem, cb),
                2.. => self.scan_prefix_2(mem, cb),
                _ => self.scan_prefix_1(mem, cb),
            }
        }

        pub fn find_anchor(&self, name: &str) -> Option<&(String, usize)> {
            self.anchors.iter().find(|x| x.0 == name)
        }
    }

    #[derive(Clone, Copy)]
    pub(crate) struct PatternByte {
        value: u8,
        mask: u8,
    }

    impl Debug for PatternByte {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&match self.mask {
                0xFF => format!("{:X}{:X}", self.value >> 4, self.value & 0x0F),
                0xF0 => format!("{:X}?", self.value >> 4),
                0x0F => format!("?{:X}", self.value & 0x0F),
                _ => return f.write_str("??"),
            })
        }
    }

    impl PatternByte {
        fn new(value: u8, mask: u8) -> Self {
            Self { value, mask }
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Condition {
        pub offset: isize,
        pub value: ConditionValue,
    }

    impl Condition {
        pub fn from_parsed(value: xml::Condition, pattern: &Pattern) -> anyhow::Result<Self> {
            let offset = calculate_offset(&value.offset, pattern)?;

            Ok(Self {
                offset,
                value: match value.r#type {
                    xml::ConditionType::String => ConditionValue::String(value.value),
                    xml::ConditionType::FixedString => ConditionValue::String(value.value),
                    xml::ConditionType::FixedStringIndirect => ConditionValue::String(value.value),
                },
            })
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) enum ConditionValue {
        String(String),
        FixedString(String),
        FixedStringIndirect(String),
    }

    #[derive(Clone, Debug)]
    pub(crate) enum TargetsOrPatch {
        Targets(Vec<Target>),
        Patch(Patch),
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Target {
        pub r#type: TargetType,
        pub offset: isize,
        pub value: TargetValue,
    }

    impl Target {
        pub fn from_parsed(value: xml::Target, pattern: &Pattern) -> anyhow::Result<Self> {
            let offset = calculate_offset(&value.offset, pattern)?;

            Ok(Self {
                r#type: value.r#type.try_into()?,
                offset,
                value: {
                    match (
                        value.symbol,
                        value.next_symbol,
                        value.next_symbol_seek_size,
                        value.engine_callback,
                    ) {
                        (Some(s), None, None, None) => TargetValue::Symbol(s),
                        (None, Some(value), Some(offset), None) => TargetValue::NextSymbol {
                            value,
                            offset: calculate_offset(&offset, pattern)?,
                        },
                        (None, None, None, Some(ec)) => TargetValue::EngineCallback(ec),
                        (symbol, next_symbol, next_symbol_seek_size, engine_callback) => {
                            bail!("unexpected target definition: {:#?}", xml::Target {
                                symbol,
                                next_symbol,
                                next_symbol_seek_size,
                                engine_callback,
                                ..value
                            })
                        }
                    }
                },
            })
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) enum TargetType {
        Absolute,
        Indirect,
    }

    impl TryFrom<xml::TargetType> for TargetType {
        type Error = anyhow::Error;

        fn try_from(value: xml::TargetType) -> Result<Self, Self::Error> {
            Ok(match value {
                xml::TargetType::Absolute => Self::Absolute,
                xml::TargetType::Indirect => Self::Indirect,
            })
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) enum TargetValue {
        Symbol(String),
        NextSymbol { value: String, offset: isize },
        EngineCallback(String),
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Patch {
        offset: isize,
        text: String,
    }

    impl Patch {
        pub fn from_parsed(value: xml::Patch, pattern: &Pattern) -> anyhow::Result<Self> {
            let offset = calculate_offset(&value.offset, pattern)?;

            Ok(Self { offset, text: value.text })
        }
    }

    fn hex_to_u8(c1: char, c2: char) -> Option<u8> {
        let c1 = c1.to_digit(16)? as u8;
        let c2 = c2.to_digit(16)? as u8;

        Some((c1 << 4) + c2)
    }

    fn calculate_offset(offset: &str, pattern: &Pattern) -> anyhow::Result<isize> {
        if offset.starts_with('@') {
            if let Some((_, offset)) = pattern.find_anchor(offset) {
                Ok(*offset as _)
            } else {
                bail!("unable to find offset {offset} in pattern")
            }
        } else {
            offset
                .trim_start_matches("0x")
                .parse()
                .map_err(|_| anyhow!("unable to parse offset {offset}"))
        }
    }
}

pub(crate) mod xml {
    #![allow(dead_code)]
    use serde::Deserialize;

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) struct BinaryMappings {
        #[serde(rename = "Mappings")]
        pub mappings: Mappings,
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) struct Mappings {
        #[serde(rename = "@Version")]
        pub version: String,
        #[serde(rename = "@Default")]
        pub default: bool,
        #[serde(rename = "$value")]
        pub inner: Vec<MappingOrDllImport>,
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) enum MappingOrDllImport {
        DllImport(DllImport),
        Mapping(Mapping),
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) struct DllImport {
        #[serde(rename = "@Module")]
        pub module: String,
        #[serde(rename = "@Proc")]
        pub proc: String,
        #[serde(rename = "@Symbol")]
        pub symbol: String,
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) struct Mapping {
        #[serde(rename = "@Name")]
        pub name: String,
        #[serde(rename = "@Critical", default)]
        pub critical: bool,
        #[serde(rename = "@AllowFail", default)]
        pub allow_fail: bool,
        #[serde(rename = "@Scope", default)]
        pub scope: MappingScope,
        #[serde(rename = "$value")]
        pub props: Vec<MappingProperty>,
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) enum MappingProperty {
        Patch(Patch),
        Target(Target),
        Condition(Condition),
        #[serde(rename = "$text")]
        Pattern(String),
    }

    #[derive(Clone, Copy, Deserialize, Debug, Default)]
    pub(crate) enum MappingScope {
        #[default]
        Text,
        Custom,
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) struct Condition {
        #[serde(rename = "@Type")]
        pub r#type: ConditionType,
        #[serde(rename = "@Offset")]
        pub offset: String,
        #[serde(rename = "@Value")]
        pub value: String,
    }

    #[derive(Clone, Copy, Deserialize, Debug)]
    pub(crate) enum ConditionType {
        String,
        FixedString,
        FixedStringIndirect,
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) struct Target {
        #[serde(rename = "@Type")]
        pub r#type: TargetType,
        #[serde(rename = "@Offset")]
        pub offset: String,
        #[serde(rename = "@Symbol")]
        pub symbol: Option<String>,
        #[serde(rename = "@NextSymbol")]
        pub next_symbol: Option<String>,
        #[serde(rename = "@NextSymbolSeekSize")]
        pub next_symbol_seek_size: Option<String>,
        #[serde(rename = "@EngineCallback")]
        pub engine_callback: Option<String>,
    }

    #[derive(Clone, Copy, Deserialize, Debug)]
    pub(crate) enum TargetType {
        Absolute,
        Indirect,
    }

    #[derive(Clone, Deserialize, Debug)]
    pub(crate) struct Patch {
        #[serde(rename = "@Type")]
        pub r#type: PatchType,
        #[serde(rename = "@Offset")]
        pub offset: String,
        #[serde(rename = "$text")]
        pub text: String,
    }

    #[derive(Clone, Copy, Deserialize, Debug)]
    pub(crate) enum PatchType {
        Absolute,
    }
}
