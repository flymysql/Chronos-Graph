//! Embedded form factor: open a Chronos-Graph engine in-process, for single-node
//! development or edge/private agent-memory deployments. Shares the same kernel
//! as the server build.

pub mod fact_codec;
pub mod fact_store;
pub mod retriever;

pub use fact_store::{CommunitySummary, FactStore};
pub use retriever::MemoryRetriever;

use chronos_common::config::EngineConfig;
use chronos_common::Result;

/// An in-process handle to the engine.
pub struct Chronos {
    pub config: EngineConfig,
    pub facts: FactStore,
}

impl Chronos {
    /// Open (or create) an embedded engine at the configured data directory.
    ///
    /// With the `rocks` feature the store is durable (RocksDB at
    /// `config.data_dir`, recovering all state on open); otherwise it is an
    /// in-memory store.
    pub fn open(config: EngineConfig) -> Result<Self> {
        #[cfg(feature = "rocks")]
        let facts = FactStore::open_rocks(&config.data_dir)?;
        #[cfg(not(feature = "rocks"))]
        let facts = FactStore::new();
        Ok(Self { config, facts })
    }
}
