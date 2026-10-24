//! Build-time decoder regression tests share the production implementation.

#[allow(dead_code)]
#[path = "../build_support/c_source.rs"]
mod c_source;
#[allow(dead_code)]
#[path = "../build_support/nutation.rs"]
mod nutation;

#[allow(dead_code)]
#[path = "../build_support/moshier.rs"]
mod moshier;

#[allow(dead_code)]
#[path = "../build_support/period_remainder.rs"]
mod period_remainder;
