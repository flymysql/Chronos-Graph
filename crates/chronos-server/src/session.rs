//! Client sessions / connection state.

#[derive(Debug, Clone)]
pub struct Session {
    pub tenant: u64,
    pub principal: String,
}
