//! Compare explicit current and 2.10.03 split contracts for stdin input rows.
//!
//! Each input is `decimal_degrees flags sidereal_mode`; output contains both
//! complete typed results. Fractional seconds are emitted as f64 bits so callers
//! can compare exact values in memory without rounding a decimal conversion.
use std::io::{self, BufRead};
use swisseph_bindings::{SplitDeg, set_sid_mode, split_deg, split_deg_2_10_03};

fn words(value: SplitDeg) -> String {
    format!(
        "{} {} {} {} {}",
        value.deg,
        value.min,
        value.sec,
        value.secfr.to_bits(),
        value.sign
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        let values: Vec<_> = line.split_whitespace().collect();
        if values.len() != 3 {
            return Err("expected decimal_degrees flags sidereal_mode".into());
        }
        let degree = values[0].parse()?;
        let flags = values[1].parse()?;
        set_sid_mode(values[2].parse()?, 0.0, 0.0)?;
        println!(
            "{} | {}",
            words(split_deg(degree, flags)?),
            words(split_deg_2_10_03(degree, flags)?)
        );
    }
    Ok(())
}
