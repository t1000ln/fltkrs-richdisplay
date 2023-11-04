use std::sync::OnceLock;

pub(crate) static ID_GENERATOR_INIT: OnceLock<u8> = OnceLock::new();