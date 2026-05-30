//! Embedded form factor: open a Chronos-Graph engine in-process, for single-node
//! development or edge/private agent-memory deployments. Shares the same kernel
//! as the server build.

use chronos_common::config::EngineConfig;
use chronos_common::Result;

/// An in-process handle to the engine.
pub struct Chronos {
    pub config: EngineConfig,
}

impl Chronos {
    /// Open (or create) an embedded engine at the configured data directory.
    pub fn open(config: EngineConfig) -> Result<Self> {
        // TODO(M1): wire storage + indexes from config.
        Ok(Self { config })
    }
}
