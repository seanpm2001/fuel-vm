use super::{
    Key,
    Table,
};

/// Registry database needs these behaviors
pub trait RegistryDb {
    /// Get next key for the given table. This is where the next write should start at.
    /// The result of this function is just a suggestion, and the caller may choose to
    /// ignore it, although it's rare that they would know better.
    fn next_key<T: Table>(&self) -> anyhow::Result<Key<T>>;

    /// Read a value from the registry by key
    fn read<T: Table>(&self, key: Key<T>) -> anyhow::Result<T::Type>;

    /// Write a continuous sequence of values to the registry
    fn batch_write<T: Table>(
        &mut self,
        start_key: Key<T>,
        values: Vec<T::Type>,
    ) -> anyhow::Result<()>;

    /// Lookup a key by value
    fn index_lookup<T: Table>(&self, value: &T::Type) -> anyhow::Result<Option<Key<T>>>;
}