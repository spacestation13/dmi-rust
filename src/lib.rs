pub mod dmi;

pub use crate::dmi::{chunk, crc, error, icon, ztxt};

#[cfg(test)]
mod tests;
