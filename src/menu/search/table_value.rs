use std::{cmp::Ordering, fmt::Debug, ops::Deref};

use imgui::Ui;
use itertools::Itertools;

use super::table::{details_view, TableItem};
use crate::game_definitions::{
    Array, CompactSet, FixedString, GameHash, GamePtr, Guid, LSStringView, MultiHashMap,
    MultiHashSet, OverrideableProperty, STDString, Set, TranslatedString,
};

pub(crate) trait GameObjectVisitor {
    type Return;
    fn visit(&mut self, name: impl AsRef<str>, item: &impl TableValue) -> Self::Return;
}

pub(crate) trait GameObjectFullVisitor: GameObjectVisitor {
    type Finish;
    fn finish(self) -> Self::Finish;
}

pub(crate) trait GameObjectParallelVisitor {
    type Return;
    fn visit_parallel<T: TableValue>(
        &mut self,
        name: impl AsRef<str>,
        a: &T,
        b: &T,
    ) -> Self::Return;
}

macro_rules! tbl_ord {
    ($type:ty) => {
        impl TableOrd for $type {
            fn tbl_cmp(&self, other: &Self) -> Ordering {
                self.cmp(other)
            }
        }
    };
}

macro_rules! tbl_val_primitive {
    ($type:ty) => {
        impl TableValue for $type {
            fn type_name() -> String {
                stringify!($type).into()
            }

            fn export_str(&self) -> String {
                self.to_string()
            }
        }

        tbl_ord!($type);
    };
}

tbl_val_primitive!(bool);
tbl_val_primitive!(u8);
tbl_val_primitive!(u16);
tbl_val_primitive!(u32);
tbl_val_primitive!(u64);
tbl_val_primitive!(usize);
tbl_val_primitive!(i8);
tbl_val_primitive!(i16);
tbl_val_primitive!(i32);
tbl_val_primitive!(i64);
tbl_val_primitive!(isize);

macro_rules! tbl_val_float {
    ($type:ty) => {
        impl TableValue for $type {
            fn type_name() -> String {
                stringify!($type).into()
            }

            fn export_str(&self) -> String {
                self.to_string()
            }
        }

        impl TableOrd for $type {
            fn tbl_cmp(&self, other: &Self) -> Ordering {
                self.total_cmp(other)
            }
        }
    };
}

tbl_val_float!(f32);
tbl_val_float!(f64);

macro_rules! tbl_ptr {
    ($mut:ident) => {
        impl<V: TableValue> TableItem for *$mut V {
            fn visit<T: GameObjectVisitor>(&self, _visitor: &mut T, _i: usize) -> T::Return {
                unimplemented!()
            }

            fn visit_field<T: GameObjectVisitor>(&self, _visitor: &mut T, _name: &str) -> Option<T::Return> {
                unimplemented!()
            }

            fn visit_all<T: GameObjectFullVisitor>(&self, mut visitor: T) -> T::Finish {
                if !self.is_null() {
                    unsafe { visitor.visit(format!("{self:?}"), &(**self)) };
                } else {
                    visitor.visit("NULL", &());
                }
                visitor.finish()
            }
        }

        impl<T: TableValue> TableValue for *$mut T {
            fn type_name() -> String {
                format!("{}*", T::type_name())
            }

            fn export_str(&self) -> String {
                if self.is_null() {
                    "NULL".into()
                } else {
                    format!("{self:?}")
                }
            }

            fn draw(&self, ui: &Ui) {
                details_view(ui, self);
            }

            fn search_str(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                unsafe {
                    if !self.is_null() {
                        (**self).search_str(f)
                    } else {
                        Ok(())
                    }
                }
            }

            fn is_defined(&self) -> bool {
                !self.is_null()
            }

            fn is_container(&self) -> bool {
                true
            }
        }

        impl<T> TableOrd for *$mut T {
            fn tbl_cmp(&self, other: &Self) -> Ordering {
                self.cmp(other)
            }
        }
    };
}

tbl_ptr!(const);
tbl_ptr!(mut);

impl TableValue for () {
    fn type_name() -> String {
        "void".into()
    }
}

impl TableOrd for () {
    fn tbl_cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl TableValue for &str {
    fn type_name() -> String {
        "String".into()
    }

    fn export_str(&self) -> String {
        self.to_string()
    }

    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self)
    }

    fn search_str(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }

    fn is_defined(&self) -> bool {
        !self.is_empty()
    }
}
tbl_ord!(&str);

impl<T: TableValue> TableValue for Option<T> {
    fn type_name() -> String {
        T::type_name()
    }

    fn export_str(&self) -> String {
        self.as_ref().map(|x| x.export_str()).unwrap_or_default()
    }

    fn draw(&self, ui: &Ui) {
        if let Some(x) = self {
            x.draw(ui)
        }
    }

    fn search_str(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(x) = self {
            return x.search_str(f);
        }
        Ok(())
    }

    fn is_defined(&self) -> bool {
        self.is_some()
    }
}

impl<T> TableOrd for Option<T>
where
    T: TableOrd,
{
    fn tbl_cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.tbl_cmp(b),
        }
    }
}

impl TableValue for Guid {
    fn type_name() -> String {
        "GUID".into()
    }

    fn export_str(&self) -> String {
        self.0
            .to_le_bytes()
            .iter()
            .chain(self.1.to_le_bytes().iter())
            .map(|x| format!("{x:02X}"))
            .join(" ")
    }
}

impl TableOrd for Guid {
    fn tbl_cmp(&self, other: &Self) -> Ordering {
        self.export_str().cmp(&other.export_str())
    }
}

impl<E: TableValue, const N: usize> TableItem for [E; N] {
    fn visit<T: GameObjectVisitor>(&self, visitor: &mut T, i: usize) -> T::Return {
        visitor.visit(i.to_string(), &self[i])
    }

    fn visit_field<T: GameObjectVisitor>(&self, visitor: &mut T, name: &str) -> Option<T::Return> {
        Some(visitor.visit(name, &self[name.parse::<usize>().ok()?]))
    }

    fn visit_all<T: GameObjectFullVisitor>(&self, mut visitor: T) -> T::Finish {
        for (i, x) in self.iter().enumerate() {
            visitor.visit(i.to_string(), x);
        }
        visitor.finish()
    }
}

impl<T: TableValue, const N: usize> TableValue for [T; N] {
    fn type_name() -> String {
        format!("[{}; {}]", T::type_name(), N)
    }

    fn draw(&self, ui: &Ui) {
        details_view(ui, self);
    }

    fn is_defined(&self) -> bool {
        !self.is_empty()
    }

    fn is_container(&self) -> bool {
        true
    }
}

impl<T: TableValue, const N: usize> TableOrd for [T; N] {
    fn tbl_cmp(&self, other: &Self) -> Ordering {
        for (a, b) in self.iter().zip(other.iter()) {
            match a.tbl_cmp(b) {
                x @ Ordering::Less => return x,
                Ordering::Equal => (),
                x @ Ordering::Greater => return x,
            }
        }
        Ordering::Equal
    }
}

impl<E: TableValue> TableItem for &[E] {
    fn visit<T: GameObjectVisitor>(&self, visitor: &mut T, i: usize) -> T::Return {
        visitor.visit(i.to_string(), &self[i])
    }

    fn visit_field<T: GameObjectVisitor>(&self, visitor: &mut T, name: &str) -> Option<T::Return> {
        Some(visitor.visit(name, &self[name.parse::<usize>().ok()?]))
    }

    fn visit_all<T: GameObjectFullVisitor>(&self, mut visitor: T) -> T::Finish {
        for (i, x) in self.iter().enumerate() {
            visitor.visit(i.to_string(), x);
        }
        visitor.finish()
    }
}

impl<T: TableValue> TableValue for &[T] {
    fn type_name() -> String {
        format!("[{}]", T::type_name())
    }

    fn draw(&self, ui: &Ui) {
        details_view(ui, self);
    }

    fn is_defined(&self) -> bool {
        !self.is_empty()
    }

    fn is_container(&self) -> bool {
        true
    }
}

impl<T: TableValue> TableOrd for &[T] {
    fn tbl_cmp(&self, other: &Self) -> Ordering {
        for (a, b) in self.iter().zip(other.iter()) {
            match a.tbl_cmp(b) {
                x @ Ordering::Less => return x,
                Ordering::Equal => (),
                x @ Ordering::Greater => return x,
            }
        }
        Ordering::Equal
    }
}

impl<E: TableValue + Eq + GameHash> TableItem for MultiHashSet<E> {
    fn visit<T: GameObjectVisitor>(&self, visitor: &mut T, i: usize) -> T::Return {
        let val = self.iter().nth(i).unwrap();
        visitor.visit(i.to_string(), val)
    }

    fn visit_field<T: GameObjectVisitor>(&self, visitor: &mut T, name: &str) -> Option<T::Return> {
        unimplemented!()
    }

    fn visit_all<T: GameObjectFullVisitor>(&self, mut visitor: T) -> T::Finish {
        for (i, x) in self.iter().enumerate() {
            visitor.visit(i.to_string(), x);
        }
        visitor.finish()
    }
}

impl<T: TableValue + Eq + GameHash> TableValue for MultiHashSet<T> {
    fn type_name() -> String {
        format!("MultiHashSet<{}>", T::type_name())
    }

    fn draw(&self, ui: &Ui) {
        details_view(ui, self);
    }

    fn is_container(&self) -> bool {
        true
    }
}

impl<T: TableValue + Eq + GameHash> TableOrd for MultiHashSet<T> {
    fn tbl_cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<K: TableValue + Eq + GameHash, V: TableValue> TableItem for MultiHashMap<K, V> {
    fn visit<T: GameObjectVisitor>(&self, visitor: &mut T, i: usize) -> T::Return {
        let (k, v) = self.entries().nth(i).unwrap();
        visitor.visit(k.export_str(), v)
    }

    fn visit_field<T: GameObjectVisitor>(&self, visitor: &mut T, name: &str) -> Option<T::Return> {
        unimplemented!()
    }

    fn visit_all<T: GameObjectFullVisitor>(&self, mut visitor: T) -> T::Finish {
        for (k, x) in self.entries() {
            visitor.visit(k.export_str(), x);
        }
        visitor.finish()
    }
}

impl<K: TableValue + Eq + GameHash, V: TableValue> TableValue for MultiHashMap<K, V> {
    fn type_name() -> String {
        format!("MultiHashMap<{}, {}>", K::type_name(), V::type_name())
    }

    fn draw(&self, ui: &Ui) {
        details_view(ui, self);
    }

    fn is_container(&self) -> bool {
        true
    }
}

impl<K: TableValue + Eq + GameHash, V: TableValue> TableOrd for MultiHashMap<K, V> {
    fn tbl_cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

macro_rules! tbl_value_delegate {
    ($($delegate:tt)*) => {
            fn export_str(&self) -> String {
                self.$($delegate)*.export_str()
            }

            fn draw(&self, ui: &Ui) {
                self.$($delegate)*.draw(ui)
            }

            fn search_str(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.$($delegate)*.search_str(f)
            }

            fn is_defined(&self) -> bool {
                self.$($delegate)*.is_defined()
            }

            fn is_container(&self) -> bool {
                self.$($delegate)*.is_container()
            }
    };
}

macro_rules! tbl_ord_delegate {
    ($($delegate:tt)*) => {
        fn tbl_cmp(&self, other: &Self) -> Ordering {
            self.$($delegate)*.tbl_cmp(&other.$($delegate)*)
        }
    };
}

macro_rules! tbl_item_delegate {
    ($($delegate:tt)*) => {
        fn visit<T: GameObjectVisitor>(&self, visitor: &mut T, i: usize) -> T::Return {
            self.$($delegate)*.visit(visitor, i)
        }

        fn visit_field<T: GameObjectVisitor>(&self, visitor: &mut T, name: &str) -> Option<T::Return> {
            self.$($delegate)*.visit_field(visitor, name)
        }

        fn visit_all<T: GameObjectFullVisitor>(&self, visitor: T) -> T::Finish {
            self.$($delegate)*.visit_all(visitor)
        }
    };
}

impl<T: TableValue> TableValue for Array<T> {
    tbl_value_delegate!(deref());

    fn type_name() -> String {
        format!("Array<{}>", T::type_name())
    }
}

impl<V: TableValue> TableItem for Array<V> {
    tbl_item_delegate!(deref());
}

impl<T: TableValue> TableOrd for Array<T> {
    tbl_ord_delegate!(deref());
}

impl<T: TableValue> TableValue for Set<T> {
    tbl_value_delegate!(deref());

    fn type_name() -> String {
        format!("Set<{}>", T::type_name())
    }
}
impl<V: TableValue> TableItem for Set<V> {
    tbl_item_delegate!(deref());
}

impl<T: TableValue> TableOrd for Set<T> {
    tbl_ord_delegate!(deref());
}

impl<T: TableValue> TableValue for CompactSet<T> {
    tbl_value_delegate!(deref());

    fn type_name() -> String {
        format!("CompactSet<{}>", T::type_name())
    }
}
impl<V: TableValue> TableItem for CompactSet<V> {
    tbl_item_delegate!(deref());
}

impl<T: TableValue> TableOrd for CompactSet<T> {
    tbl_ord_delegate!(deref());
}

impl TableValue for String {
    tbl_value_delegate!(as_str());

    fn type_name() -> String {
        "String".into()
    }
}

impl TableOrd for String {
    tbl_ord_delegate!(as_str());
}

impl<T: TableValue> TableValue for OverrideableProperty<T> {
    tbl_value_delegate!(value);

    fn type_name() -> String {
        T::type_name()
    }
}

impl<T: TableValue> TableOrd for OverrideableProperty<T> {
    tbl_ord_delegate!(value);
}

impl<V: TableValue> TableItem for GamePtr<V> {
    tbl_item_delegate!(ptr);
}

impl<T: TableValue> TableValue for GamePtr<T> {
    tbl_value_delegate!(ptr);

    fn type_name() -> String {
        format!("{}*", T::type_name())
    }
}

impl<T: TableValue> TableOrd for GamePtr<T> {
    tbl_ord_delegate!(ptr);
}

impl TableValue for FixedString {
    tbl_value_delegate!(get());

    fn type_name() -> String {
        "FixedString".into()
    }
}

impl TableOrd for FixedString {
    tbl_ord_delegate!(get());
}

impl TableValue for LSStringView<'_> {
    tbl_value_delegate!(as_str());

    fn type_name() -> String {
        "ls::StringView".into()
    }
}

impl TableOrd for LSStringView<'_> {
    tbl_ord_delegate!(as_str());
}

impl TableValue for STDString {
    tbl_value_delegate!(as_str());

    fn type_name() -> String {
        "std::string".into()
    }
}

impl TableOrd for STDString {
    tbl_ord_delegate!(as_str());
}

impl TableValue for TranslatedString {
    tbl_value_delegate!(get());

    fn type_name() -> String {
        "TranslatedString".into()
    }
}

impl TableOrd for TranslatedString {
    tbl_ord_delegate!(get());
}

pub(crate) trait TableValue: TableOrd + Debug {
    fn type_name() -> String;
    fn export_str(&self) -> String {
        format!("{self:#?}")
    }
    fn search_str(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
    fn draw(&self, ui: &Ui) {
        ui.text_wrapped(self.export_str())
    }
    fn is_defined(&self) -> bool {
        true
    }
    fn is_container(&self) -> bool {
        false
    }
}

pub(crate) trait TableOrd {
    fn tbl_cmp(&self, other: &Self) -> Ordering;
}
