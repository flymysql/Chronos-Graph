//! Embedded form factor: open a Chronos-Graph engine in-process, for single-node
//! development or edge/private agent-memory deployments. Shares the same kernel
//! as the server build.

pub mod fact_codec;
pub mod fact_store;

pub use fact_store::FactStore;

use chronos_common::config::EngineConfig;
use chronos_common::Result;

/// An in-process handle to the engine.
pub struct Chronos {
    pub config: EngineConfig,
    pub facts: FactStore,
}

impl Chronos {
    /// Open (or create) an embedded engine at the configured data directory.
    pub fn open(config: EngineConfig) -> Result<Self> {
        // M1: in-memory FactStore. A durable RocksDB-backed store (behind the
        // storage `rocks` feature) will be selectable via config in M1+.
        Ok(Self {
            config,
            facts: FactStore::new(),
        })
    }
}
