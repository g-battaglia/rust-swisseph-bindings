//! Primary node/apse transport, independent of native legacy projections.

/// Four complete node/apse vectors in the requested frame and units.
///
/// Spherical defaults are longitude/latitude in degrees and distance in AU,
/// followed by per-day rates. XYZ/radian flags select their documented units.
/// The aphelion field retains the second focus when FOPOINT is requested.
/// Undefined native components, including NaN payloads, are never reconstructed.
#[derive(Debug, Clone, PartialEq)]
pub struct NodesApsides {
    /// Ascending node: all six position/rate components.
    pub ascending: [f64; 6],
    /// Descending node: all six position/rate components.
    pub descending: [f64; 6],
    /// Perihelion: all six position/rate components.
    pub perihelion: [f64; 6],
    /// Aphelion or second focus: all six position/rate components.
    pub aphelion: [f64; 6],
    /// Actual native integer status, not requested/returned source flags.
    /// None means the producer did not provide a status channel.
    pub status: Option<i32>,
    /// Original diagnostic, including Some("") when a channel was supplied.
    /// None means no diagnostic channel; it is not an inferred clean success.
    pub diagnostic: Option<String>,
}

impl NodesApsides {
    /// Copy the actual native channels without converting status into flags.
    pub(crate) fn from_native(value: crate::observer::NodesApsides) -> Self {
        Self {
            ascending: value.ascending,
            descending: value.descending,
            perihelion: value.perihelion,
            aphelion: value.aphelion,
            status: Some(value.returned_flags),
            diagnostic: Some(value.diagnostic),
        }
    }
}
