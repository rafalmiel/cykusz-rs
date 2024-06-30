macro_rules! or_default {
    // :tt would work but intellij fails to infer types
    (type $default:ty) => {
        $default
    };
    (type $_default:ty, $value:ty) => {
        $value
    };
    (expr $default:expr) => {
        $default
    };
    (expr $_default:expr, $value:expr) => {
        $value
    };
}
macro_rules! impl_get_method {
    ($name:ident, $f:expr => bool) => {
        pub fn $name(&self) -> bool {
            self.is_set($f)
        }
    };
    ($name:ident, $f:expr => $out:ty) => {
        pub fn $name(&self) -> $out {
            self.read($f).into()
        }
    };
    ($name:ident, $f:expr => enum $out:ty ) => {
        pub fn $name(&self) -> Option<$out> {
            self.read_as_enum($f)
        }
    };
}

macro_rules! impl_set_method {
    ($name:ident, $field:expr => as $utyp:ty) => {
        pub fn $name(&mut self, value: $utyp) {
            self.modify($field.val(value as $utyp))
        }
    };
    ($name:ident, $field:expr => into $val_type:ty) => {
        pub fn $name(&mut self, value: $val_type) {
            self.modify($field.val(value.into()))
        }
    };
    ($name:ident, enum $class:path, $field:ty => as $utyp:ty) => {
        pub fn $name(&mut self, value: $field) {
            use $class as base;
            self.modify(base.val(value as $utyp))
        }
    };
    ($name:ident, bool $class:path) => {
        pub fn $name(&mut self, value: bool) {
            use $class as base;
            self.modify(if value { base::SET } else { base::CLEAR });
        }
    };
}

macro_rules! impl_const_set_method {
    ($name:ident, $field:expr => as $utyp:ty) => {
        pub fn $name(&self, value: $utyp) {
            self.modify($field.val(value as $utyp))
        }
    };
    ($name:ident, $field:expr => into $val_type:ty) => {
        pub fn $name(&self, value: $val_type) {
            self.modify($field.val(value.into()))
        }
    };
    ($name:ident, enum $class:path, $field:ty => as $utyp:ty) => {
        pub fn $name(&self, value: $field) {
            use $class as base;
            self.modify(base.val(value as $utyp))
        }
    };
    ($name:ident, bool $class:path) => {
        pub fn $name(&self, value: bool) {
            use $class as base;
            self.modify(if value { base::SET } else { base::CLEAR });
        }
    };
}

macro_rules! impl_ints {
    (mut set $name:ident($f:expr => as $utyp:ty)) => {
        paste::paste! {
            impl_set_method!([<set_ $name>], $f => as $utyp);
        }
    };
    (const set $name:ident($f:expr => as $utyp:ty)) => {
        paste::paste! {
            impl_const_set_method!([<set_ $name>], $f => as $utyp);
        }
    };
    ($cm:ident get $name:ident($f:expr => as $utyp:ty)) => {
        impl_get_method!($name, $f => $utyp);
    };
    ($cm:ident get_set $name:ident($f:expr => as $utyp:ty)) => {
        impl_ints!($cm get $name($f => as $utyp));
        impl_ints!($cm set $name($f => as $utyp));
    };
    (
        $cm:ident $mode:ident $class:ident as $utyp:ty [
            $($name:ident($f:ident);)*
        ]
    ) => {
        $(
            impl_ints!($cm $mode $name($class::$f => as $utyp));
        )*
    }
}

macro_rules! impl_bools {
    (mut set $name:ident($f:expr)) => {
        paste::paste! {
            impl_set_method!([<set_ $name>], bool $f);
        }
    };
    (const set $name:ident($f:expr)) => {
        paste::paste! {
            impl_const_set_method!([<set_ $name>], bool $f);
        }
    };
    ($cm:ident get $name:ident($f:expr)) => {
        impl_get_method!($name, $f => bool);
    };
    ($cm:ident get_set $name:ident($f:expr)) => {
        impl_bools!($cm get $name($f));
        impl_bools!($cm set $name($f));
    };
    (
        $cm:ident $mode:ident $class:ident [
            $($name:ident($f:ident);)*
        ]
    ) => {
        $(
            impl_bools!($cm $mode $name($class::$f));
        )*
    }
}

macro_rules! impl_enums {
    (mut set $name:ident($f:ident::$f2:ident => as $utyp:ty)) => {
        paste::paste! {
            impl_set_method!([<set_ $name>], enum $f::$f2, $f::$f2::Value => as $utyp);
        }
    };
    (const set $name:ident($f:ident::$f2:ident => as $utyp:ty)) => {
        paste::paste! {
            impl_const_set_method!([<set_ $name>], enum $f::$f2, $f::$f2::Value => as $utyp);
        }
    };
    ($cm:ident get $name:ident($f:ident::$f2:ident => as $utyp:ty)) => {
        impl_get_method!($name, $f::$f2 => enum $f::$f2::Value);
    };
    ($cm:ident get_set $name:ident($f:ident::$f2:ident => as $utyp:ty)) => {
        impl_enums!($cm get $name($f::$f2 => as $utyp));
        impl_enums!($cm set $name($f::$f2 => as $utyp));
    };
    (
        $cm:ident $mode:ident $class:ident as $utyp:ty [
            $($name:ident($f:ident);)*
        ]
    ) => {
        $(
            impl_enums!($cm $mode $name($class::$f => as $utyp));
        )*
    };
}

macro_rules! impl_get_types {
    (
        $class:ident [
            $($name:ident($f:ident) => $out:ty;)*
        ]
    ) => {
        $(
            impl_get_method!($name, $class::$f => $out);
        )*
    };
}

macro_rules! impl_methods {
    ($cm:ident int $mode:ident $class:ident as $utyp:ty [
        $($rest:tt)*
    ]) => (
        impl_ints!($cm $mode $class as $utyp [
            $($rest)*
        ]);
    );
    ($cm:ident bool $mode:ident $class:ident as $utyp:ty [
        $($rest:tt)*
    ]) => (
        impl_bools!($cm $mode $class [
            $($rest)*
        ]);
    );
    ($cm:ident enum $mode:ident $class:ident as $utyp:ty [
        $($rest:tt)*
    ]) => (
        impl_enums!($cm $mode $class as $utyp [
            $($rest)*
        ]);
    );
    ($cm:ident type get $class:ident as $utyp:ty [
        $($rest:tt)*
    ]) => (
        impl_get_types!($class [
            $($rest)*
        ]);
    );
    (
        $cm:ident $class:ident as $utyp:ty,

        $($kind:ident $mode:ident [
            $($rest:tt)*
        ]),*$(,)?) => (
        $(
            impl_methods!($cm $kind $mode $class as $utyp [
                $($rest)*
            ]);
        )*
    );
}

macro_rules! impl_wrap {
    (
        [$typ:ident $(,$types:ident)+],
        $class:ident as $utyp:ty,
        $($rest:tt)*
    ) => (
        impl_wrap!(@impl $typ, $class as $utyp, $($rest)*);
        impl_wrap!([$($types)+], $class as $utyp, $($rest)*);
    );
    (
        [$typ:ident],
        $class:ident as $utyp:ty,
        $($rest:tt)*
    ) => (
        impl_wrap!(@impl $typ, $class as $utyp, $($rest)*);
    );
    (@impl WrapLocal, $class:ident as $utyp:ty, $($rest:tt)*) => {
        impl WrapLocal<$utyp, $class::Register> {
            impl_methods! {
                mut $class as $utyp,

                $($rest)*
            }
        }
    };
    (@impl $typ:ident, $class:ident as $utyp:ty, $($rest:tt)*) => {
        impl $typ<$utyp, $class::Register> {
            impl_methods! {
                const $class as $utyp,

                $($rest)*
            }
        }
    };
}
