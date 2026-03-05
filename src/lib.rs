mod sysproxy;

pub use sysproxy::{Autoproxy, Error, Result, Sysproxy};

#[cfg(feature = "utils")]
pub use sysproxy::utils;
