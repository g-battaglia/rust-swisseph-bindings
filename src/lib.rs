//! Typed Rust bindings to the pinned native Swiss Ephemeris engine.
//!
//! Configured calculations run through [`Session`] with a complete owned
//! configuration under one native lock. Pure utilities are independent of
//! configuration. [`global`] and [`native`] retain the original SDK entry
//! points, argument ordering and output transports.
//!
//! The binding forwards every numerical calculation to the native engine.
//! Full output arrays and actual status/diagnostic channels are retained;
//! absent producer channels remain absent rather than inferred from prose.

mod advisory;
mod angles;
mod armc;
mod config;
mod constants;
mod domain;
mod error;
mod events;
mod ffi;
pub mod global;
mod houses;
mod legacy;
pub mod native;
mod node_result;
mod observer;
mod positions;
mod primary;
mod pure_calendar;
mod session;
mod stars;
mod state;
#[cfg(test)]
#[path = "../tests/data_dir.rs"]
mod test_data;
mod time;
mod version;
pub use primary::*;
