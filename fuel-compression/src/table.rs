use serde::{
    Deserialize,
    Serialize,
};

use crate::Key;

mod _private {
    pub trait Seal {}
}

/// Table in the registry
pub trait Table: _private::Seal {
    /// Unique name of the table
    const NAME: &'static str;

    /// A `CountPerTable` for this table
    fn count(n: usize) -> CountPerTable;

    /// The type stored in the table
    type Type: PartialEq + Default + Serialize + for<'de> Deserialize<'de>;
}

pub mod access {
    pub trait AccessCopy<T, V: Copy> {
        fn value(&self) -> V;
    }

    pub trait AccessRef<T, V> {
        fn get(&self) -> &V;
    }

    pub trait AccessMut<T, V> {
        fn get_mut(&mut self) -> &mut V;
    }
}

macro_rules! tables {
    // $index muse use increasing numbers starting from zero
    ($($name:ident: $ty:ty),*$(,)?) => { paste::paste! {
        /// Marker struct for each table type
        pub mod tables {
            $(
                /// Specifies the table to use for a given key.
                /// The data is separated to tables based on the data type being stored.
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
                pub struct $name;

                impl super::_private::Seal for $name {}
                impl super::Table for $name {
                    const NAME: &'static str = stringify!($name);
                    fn count(n: usize) -> super::CountPerTable {
                        super::CountPerTable::$name(n)
                    }
                    type Type = $ty;
                }

                impl $name {
                    /// Calls the `to_key_*` method for this table on the context
                    pub fn to_key(value: $ty, ctx: &mut dyn super::CompactionContext) -> anyhow::Result<super::Key<$name>> {
                        ctx.[<to_key_ $name>](value)
                    }

                    /// Calls the `read_*` method for this table on the context
                    pub fn read(key: super::Key<$name>, ctx: &dyn super::DecompactionContext) -> anyhow::Result<$ty> {
                        ctx.[<read_ $name>](key)
                    }
                }
            )*
        }

        /// Context for compaction, i.e. converting data to reference-based format.
        /// The context is used to aggreage changes to the registry.
        /// A new context should be created for each compaction "session",
        /// typically a blockchain block.
        #[allow(non_snake_case)] // The field names match table type names eactly
        pub trait CompactionContext {
            $(
                /// Store a value to the changeset and return a short reference key to it.
                /// If the value already exists in the registry and will not be overwritten,
                /// the existing key can be returned instead.
                fn [<to_key_  $name>](&mut self, value: $ty) -> anyhow::Result<Key<tables::$name>>;
            )*
        }

        /// Context for compaction, i.e. converting data to reference-based format
        #[allow(non_snake_case)] // The field names match table type names eactly
        pub trait DecompactionContext {
            $(
                /// Read a value from the registry based on the key.
                fn [<read_  $name>](&self, key: Key<tables::$name>) -> anyhow::Result<<tables::$name as Table>::Type>;
            )*
        }

        /// One counter per table
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
        #[allow(non_snake_case)] // The field names match table type names eactly
        #[allow(missing_docs)] // Makes no sense to document the fields
        #[non_exhaustive]
        pub struct CountPerTable {
            $(pub $name: usize),*
        }

        impl CountPerTable {$(
            /// Custom constructor per table
            #[allow(non_snake_case)] // The field names match table type names eactly
            pub fn $name(value: usize) -> Self {
                Self {
                    $name: value,
                    ..Self::default()
                }
            }
        )*}

        $(
            impl access::AccessCopy<tables::$name, usize> for CountPerTable {
                fn value(&self) -> usize {
                    self.$name
                }
            }
        )*

        impl core::ops::Add<CountPerTable> for CountPerTable {
            type Output = Self;

            fn add(self, rhs: CountPerTable) -> Self::Output {
                Self {
                    $($name: self.$name + rhs.$name),*
                }
            }
        }

        impl core::ops::AddAssign<CountPerTable> for CountPerTable {
            fn add_assign(&mut self, rhs: CountPerTable) {
                $(self.$name += rhs.$name);*
            }
        }

        /// One key value per table
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[allow(non_snake_case)] // The field names match table type names eactly
        #[allow(missing_docs)] // Makes no sense to document the fields
        #[non_exhaustive]
        pub struct KeyPerTable {
            $(pub $name: Key<tables::$name>),*
        }

        impl Default for KeyPerTable {
            fn default() -> Self {
                Self {
                    $($name: Key::ZERO,)*
                }
            }
        }

        $(
            impl access::AccessCopy<tables::$name, Key<tables::$name>> for KeyPerTable {
                fn value(&self) -> Key<tables::$name> {
                    self.$name
                }
            }
            impl access::AccessRef<tables::$name, Key<tables::$name>> for KeyPerTable {
                fn get(&self) -> &Key<tables::$name> {
                    &self.$name
                }
            }
            impl access::AccessMut<tables::$name, Key<tables::$name>> for KeyPerTable {
                fn get_mut(&mut self) -> &mut Key<tables::$name> {
                    &mut self.$name
                }
            }
        )*

        /// Used to add together keys and counts to deterimine possible overwrite range
        pub fn add_keys(keys: KeyPerTable, counts: CountPerTable) -> KeyPerTable {
            KeyPerTable {
                $(
                    $name: keys.$name.add_u32(counts.$name.try_into()
                        .expect("Count too large. Shoudn't happen as we control inputs here.")
                    ),
                )*
            }
        }
    }};
}

tables!(
    AssetId: [u8; 32],
    Address: [u8; 32],
    ContractId: [u8; 32],
    ScriptCode: Vec<u8>,
    Witness: Vec<u8>,
);
