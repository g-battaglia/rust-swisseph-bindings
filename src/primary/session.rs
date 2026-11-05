//! Owned primary session: one configuration installation and native call.
use super::*;

/// Complete owned configuration for primary operations.
/// Native file selection belongs to the SDK; common settings never inherit
/// process-global values. Construction performs no calculation or native write.
#[derive(Clone, Debug, Default)]
pub struct SessionBuilder {
    native: crate::session::SessionBuilder,
}
/// Configured native access, serialized for each complete operation.
/// Coordinates use native frames/flags, angles degrees and distances AU unless
/// the operation requests another native representation. No numerical fallback
/// is introduced by this transport layer.
#[derive(Clone, Debug)]
pub struct Session {
    native: crate::session::Session,
}
impl SessionBuilder {
    /// Create a builder with documented owned native defaults.
    pub fn new() -> Self {
        Self::default()
    }
    /// Validate owned inputs and capture external lookup context.
    ///
    /// # Errors
    /// Invalid native configuration or unavailable lookup context is retained.
    pub fn build(&self) -> Result<Session, Error> {
        self.native
            .build_owned()
            .map(|native| Session { native })
            .map_err(Error::from_native)
    }

    /// Set owned topo inputs; validation occurs at build.
    pub fn topo(&mut self, longitude: f64, latitude: f64, height: f64) -> &mut Self {
        self.native.topo(longitude, latitude, height);
        self
    }
    /// Set owned sid mode inputs; validation occurs at build.
    pub fn sid_mode(&mut self, mode: i32, epoch: f64, offset: f64) -> &mut Self {
        self.native.sid_mode(mode, epoch, offset);
        self
    }
    /// Set owned tid acc inputs; validation occurs at build.
    pub fn tid_acc(&mut self, value: f64) -> &mut Self {
        self.native.tid_acc(value);
        self
    }
    /// Set owned delta t override inputs; validation occurs at build.
    pub fn delta_t_override(&mut self, days: f64) -> &mut Self {
        self.native.delta_t_override(days);
        self
    }
    /// Set owned clear delta t override inputs; validation occurs at build.
    pub fn clear_delta_t_override(&mut self) -> &mut Self {
        self.native.clear_delta_t_override();
        self
    }
    /// Set owned lapse rate inputs; validation occurs at build.
    pub fn lapse_rate(&mut self, value: f64) -> &mut Self {
        self.native.lapse_rate(value);
        self
    }
}
impl Session {
    /// Execute native `calc` using this complete owned configuration.
    ///
    /// See [`crate::global::calc`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Position result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn calc(&self, jd: f64, body: i32, flags: i32) -> Result<Position, Error> {
        let v = self
            .native
            .compute(|| crate::positions::calc_locked(jd, body, flags))
            .map_err(Error::from_native)?;
        Ok(position(v))
    }

    /// Execute native `calc_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::calc_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Position result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn calc_ut(&self, jd: f64, body: i32, flags: i32) -> Result<Position, Error> {
        let v = self
            .native
            .compute(|| crate::positions::calc_ut_locked(jd, body, flags))
            .map_err(Error::from_native)?;
        Ok(position(v))
    }

    /// Execute native `pheno` using this complete owned configuration.
    ///
    /// See [`crate::global::pheno`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Phenomena result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn pheno(&self, jd: f64, body: i32, flags: i32) -> Result<Phenomena, Error> {
        let v = self
            .native
            .compute(|| crate::observer::pheno_locked(jd, body, flags))
            .map_err(Error::from_native)?;
        Ok(Phenomena {
            values: v.values,
            returned_flags: v.returned_flags,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `pheno_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::pheno_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Phenomena result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn pheno_ut(&self, jd: f64, body: i32, flags: i32) -> Result<Phenomena, Error> {
        let v = self
            .native
            .compute(|| crate::observer::pheno_ut_locked(jd, body, flags))
            .map_err(Error::from_native)?;
        Ok(Phenomena {
            values: v.values,
            returned_flags: v.returned_flags,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `nod_aps` using this complete owned configuration.
    ///
    /// See [`crate::global::nod_aps`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `method` - Native node/apse method bits, including second-focus selection.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete NodesApsides result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn nod_aps(
        &self,
        jd: f64,
        body: i32,
        method: i32,
        flags: i32,
    ) -> Result<NodesApsides, Error> {
        let v = self
            .native
            .compute(|| crate::observer::nod_aps_locked(jd, body, flags, method))
            .map_err(Error::from_native)?;
        Ok(NodesApsides {
            ascending: v.ascending,
            descending: v.descending,
            perihelion: v.perihelion,
            aphelion: v.aphelion,
            status: Some(v.returned_flags),
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `nod_aps_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::nod_aps_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `method` - Native node/apse method bits, including second-focus selection.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete NodesApsides result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn nod_aps_ut(
        &self,
        jd: f64,
        body: i32,
        method: i32,
        flags: i32,
    ) -> Result<NodesApsides, Error> {
        let v = self
            .native
            .compute(|| crate::observer::nod_aps_ut_locked(jd, body, flags, method))
            .map_err(Error::from_native)?;
        Ok(NodesApsides {
            ascending: v.ascending,
            descending: v.descending,
            perihelion: v.perihelion,
            aphelion: v.aphelion,
            status: Some(v.returned_flags),
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `get_orbital_elements` using this complete owned configuration.
    ///
    /// See [`crate::global::get_orbital_elements`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete OrbitalElements result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn get_orbital_elements(
        &self,
        jd: f64,
        body: i32,
        flags: i32,
    ) -> Result<OrbitalElements, Error> {
        let v = self
            .native
            .compute(|| crate::observer::get_orbital_elements_complete_locked(jd, body, flags))
            .map_err(Error::from_native)?;
        Ok(OrbitalElements {
            values: v.0.values,
            status: Some(v.1),
            diagnostic: Some(v.0.diagnostic),
        })
    }

    /// Execute native `orbit_max_min_true_distance` using this complete owned configuration.
    ///
    /// See [`crate::global::orbit_max_min_true_distance`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete DistanceExtremes result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn orbit_max_min_true_distance(
        &self,
        jd: f64,
        body: i32,
        flags: i32,
    ) -> Result<DistanceExtremes, Error> {
        let v = self
            .native
            .compute(|| {
                crate::observer::orbit_max_min_true_distance_complete_locked(jd, body, flags)
            })
            .map_err(Error::from_native)?;
        Ok(DistanceExtremes {
            maximum: v.0.max_distance,
            minimum: v.0.min_distance,
            current: v.0.true_distance,
            status: Some(v.1),
            diagnostic: Some(v.0.diagnostic),
        })
    }

    /// Execute native `calc_pctr` using this complete owned configuration.
    ///
    /// See [`crate::global::calc_pctr`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `center` - Native center-body identifier for the planetocentric frame.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Position result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn calc_pctr(
        &self,
        jd: f64,
        body: i32,
        center: i32,
        flags: i32,
    ) -> Result<Position, Error> {
        let v = self
            .native
            .compute(|| crate::positions::calc_pctr_locked(jd, body, center, flags))
            .map_err(Error::from_native)?;
        Ok(position(v))
    }

    /// Execute native `get_planet_name` using this complete owned configuration.
    ///
    /// See [`crate::global::get_planet_name`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `body` - Native body identifier; data availability follows the selected provider.
    ///
    /// # Returns
    ///
    /// Complete Text result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn get_planet_name(&self, body: i32) -> Result<Text, Error> {
        let v = self
            .native
            .compute(|| crate::positions::get_planet_name_locked(body))
            .map_err(Error::from_native)?;
        Ok(Text { value: v })
    }

    /// Execute native `fixstar` using this complete owned configuration.
    ///
    /// See [`crate::global::fixstar`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `key` - Fixed-star query accepted by the native catalogue reader.
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete StarPosition result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn fixstar(&self, key: &str, jd: f64, flags: i32) -> Result<StarPosition, Error> {
        let v = self
            .native
            .compute(|| crate::stars::fixstar_locked(key, jd, flags))
            .map_err(Error::from_native)?;
        Ok(StarPosition {
            values: v.values,
            canonical_name: v.resolved_name,
            returned_flags: v.returned_flags,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `fixstar_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::fixstar_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `key` - Fixed-star query accepted by the native catalogue reader.
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete StarPosition result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn fixstar_ut(&self, key: &str, jd: f64, flags: i32) -> Result<StarPosition, Error> {
        let v = self
            .native
            .compute(|| crate::stars::fixstar_ut_locked(key, jd, flags))
            .map_err(Error::from_native)?;
        Ok(StarPosition {
            values: v.values,
            canonical_name: v.resolved_name,
            returned_flags: v.returned_flags,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `fixstar2` using this complete owned configuration.
    ///
    /// See [`crate::global::fixstar2`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `key` - Fixed-star query accepted by the native catalogue reader.
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete StarPosition result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn fixstar2(&self, key: &str, jd: f64, flags: i32) -> Result<StarPosition, Error> {
        let v = self
            .native
            .compute(|| crate::stars::fixstar2_locked(key, jd, flags))
            .map_err(Error::from_native)?;
        Ok(StarPosition {
            values: v.values,
            canonical_name: v.resolved_name,
            returned_flags: v.returned_flags,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `fixstar2_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::fixstar2_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `key` - Fixed-star query accepted by the native catalogue reader.
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete StarPosition result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn fixstar2_ut(&self, key: &str, jd: f64, flags: i32) -> Result<StarPosition, Error> {
        let v = self
            .native
            .compute(|| crate::stars::fixstar2_ut_locked(key, jd, flags))
            .map_err(Error::from_native)?;
        Ok(StarPosition {
            values: v.values,
            canonical_name: v.resolved_name,
            returned_flags: v.returned_flags,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `fixstar_mag` using this complete owned configuration.
    ///
    /// See [`crate::global::fixstar_mag`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `key` - Fixed-star query accepted by the native catalogue reader.
    ///
    /// # Returns
    ///
    /// Complete StarMagnitude result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn fixstar_mag(&self, key: &str) -> Result<StarMagnitude, Error> {
        let v = self
            .native
            .compute(|| crate::stars::fixstar_mag_locked(key))
            .map_err(Error::from_native)?;
        Ok(StarMagnitude {
            magnitude: v.magnitude,
            canonical_name: Some(v.resolved_name),
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `fixstar2_mag` using this complete owned configuration.
    ///
    /// See [`crate::global::fixstar2_mag`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `key` - Fixed-star query accepted by the native catalogue reader.
    ///
    /// # Returns
    ///
    /// Complete StarMagnitude result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn fixstar2_mag(&self, key: &str) -> Result<StarMagnitude, Error> {
        let v = self
            .native
            .compute(|| crate::stars::fixstar2_mag_locked(key))
            .map_err(Error::from_native)?;
        Ok(StarMagnitude {
            magnitude: v.magnitude,
            canonical_name: Some(v.resolved_name),
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `houses` using this complete owned configuration.
    ///
    /// See [`crate::global::houses`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `latitude` - Geographic north latitude in degrees.
    /// * `longitude` - Geographic east longitude in degrees.
    /// * `system` - ASCII house-system letter; fixed twelve-cusp operations reject Gauquelin.
    ///
    /// # Returns
    ///
    /// Complete Houses result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn houses(
        &self,
        jd: f64,
        latitude: f64,
        longitude: f64,
        system: u8,
    ) -> Result<Houses, Error> {
        let v = self
            .native
            .compute(|| crate::houses::houses_complete_locked(jd, latitude, longitude, system))
            .map_err(Error::from_native)?;
        Ok(Houses {
            cusps: v.0.cusps,
            angles: v.0.angles,
            status: Some(v.1),
            diagnostic: v.2,
        })
    }

    /// Execute native `houses_ex` using this complete owned configuration.
    ///
    /// See [`crate::global::houses_ex`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `latitude` - Geographic north latitude in degrees.
    /// * `longitude` - Geographic east longitude in degrees.
    /// * `system` - ASCII house-system letter; fixed twelve-cusp operations reject Gauquelin.
    ///
    /// # Returns
    ///
    /// Complete Houses result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn houses_ex(
        &self,
        jd: f64,
        flags: i32,
        latitude: f64,
        longitude: f64,
        system: u8,
    ) -> Result<Houses, Error> {
        let v = self
            .native
            .compute(|| {
                crate::houses::houses_ex_complete_locked(jd, flags, latitude, longitude, system)
            })
            .map_err(Error::from_native)?;
        Ok(Houses {
            cusps: v.0.cusps,
            angles: v.0.angles,
            status: Some(v.1),
            diagnostic: v.2,
        })
    }

    /// Execute native `houses_ex2` using this complete owned configuration.
    ///
    /// See [`crate::global::houses_ex2`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `latitude` - Geographic north latitude in degrees.
    /// * `longitude` - Geographic east longitude in degrees.
    /// * `system` - ASCII house-system letter; fixed twelve-cusp operations reject Gauquelin.
    ///
    /// # Returns
    ///
    /// Complete HousesWithSpeeds result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn houses_ex2(
        &self,
        jd: f64,
        flags: i32,
        latitude: f64,
        longitude: f64,
        system: u8,
    ) -> Result<HousesWithSpeeds, Error> {
        let v = self
            .native
            .compute(|| {
                crate::houses::houses_ex2_complete_locked(jd, flags, latitude, longitude, system)
            })
            .map_err(Error::from_native)?;
        Ok(HousesWithSpeeds {
            cusps: v.0.cusps,
            angles: v.0.angles,
            cusp_speeds: v.0.cusp_speeds,
            angle_speeds: v.0.angle_speeds,
            status: Some(v.1),
            diagnostic: v.2,
        })
    }

    /// Execute native `houses_gauquelin` using this complete owned configuration.
    ///
    /// See [`crate::global::houses_gauquelin`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `latitude` - Geographic north latitude in degrees.
    /// * `longitude` - Geographic east longitude in degrees.
    ///
    /// # Returns
    ///
    /// Complete GauquelinSectors result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn houses_gauquelin(
        &self,
        jd: f64,
        flags: i32,
        latitude: f64,
        longitude: f64,
    ) -> Result<GauquelinSectors, Error> {
        let v = self
            .native
            .compute(|| {
                crate::houses::houses_gauquelin_complete_locked(jd, flags, latitude, longitude)
            })
            .map_err(Error::from_native)?;
        Ok(GauquelinSectors {
            sectors: v.0.sectors,
            angles: v.0.angles,
            sector_speeds: v.0.sector_speeds,
            angle_speeds: v.0.angle_speeds,
            status: Some(v.1),
            diagnostic: v.2,
        })
    }

    /// Execute native `houses_armc` using this complete owned configuration.
    ///
    /// See [`crate::global::houses_armc`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `armc` - Right ascension of the Midheaven in degrees.
    /// * `latitude` - Geographic north latitude in degrees.
    /// * `obliquity` - Ecliptic obliquity in degrees.
    /// * `system` - ASCII house-system letter; fixed twelve-cusp operations reject Gauquelin.
    ///
    /// # Returns
    ///
    /// Complete ArmcHouses result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn houses_armc(
        &self,
        armc: f64,
        latitude: f64,
        obliquity: f64,
        system: u8,
    ) -> Result<ArmcHouses, Error> {
        fn run<const N: usize>(
            armc: f64,
            latitude: f64,
            obliquity: f64,
            system: u8,
        ) -> Result<ArmcHouses, crate::error::Error> {
            let (cusps, angles, status) =
                crate::armc::values::<N>(armc, latitude, obliquity, system)?;
            Ok(ArmcHouses {
                cusps: cusps[1..].to_vec(),
                angles: angles[..8].try_into().expect("native angle width"),
                status: Some(status),
                diagnostic: None,
            })
        }
        if system.eq_ignore_ascii_case(&b'G') {
            self.native
                .compute(|| run::<37>(armc, latitude, obliquity, system))
                .map_err(Error::from_native)
        } else {
            self.native
                .compute(|| run::<13>(armc, latitude, obliquity, system))
                .map_err(Error::from_native)
        }
    }

    /// Execute native `houses_armc_ex2` using this complete owned configuration.
    ///
    /// See [`crate::global::houses_armc_ex2`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `armc` - Right ascension of the Midheaven in degrees.
    /// * `latitude` - Geographic north latitude in degrees.
    /// * `obliquity` - Ecliptic obliquity in degrees.
    /// * `system` - ASCII house-system letter; fixed twelve-cusp operations reject Gauquelin.
    /// * `sun_declination` - Solar declination in degrees for Sunshine; otherwise ignored natively.
    ///
    /// # Returns
    ///
    /// Complete ArmcHousesWithSpeeds result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn houses_armc_ex2(
        &self,
        armc: f64,
        latitude: f64,
        obliquity: f64,
        system: u8,
        sun_declination: f64,
    ) -> Result<ArmcHousesWithSpeeds, Error> {
        fn run<const N: usize>(
            armc: f64,
            latitude: f64,
            obliquity: f64,
            system: u8,
            sun_declination: f64,
        ) -> Result<ArmcHousesWithSpeeds, crate::error::Error> {
            let v = crate::armc::rates::<N>(armc, latitude, obliquity, system, sun_declination)?;
            Ok(ArmcHousesWithSpeeds {
                cusps: v.cusps[1..].to_vec(),
                angles: v.angles[..8].try_into().expect("native angle width"),
                cusp_speeds: v.cusp_speeds[1..].to_vec(),
                angle_speeds: v.angle_speeds[..8].try_into().expect("native angle width"),
                status: Some(v.status),
                diagnostic: Some(v.diagnostic),
            })
        }
        if system.eq_ignore_ascii_case(&b'G') {
            self.native
                .compute(|| run::<37>(armc, latitude, obliquity, system, sun_declination))
                .map_err(Error::from_native)
        } else {
            self.native
                .compute(|| run::<13>(armc, latitude, obliquity, system, sun_declination))
                .map_err(Error::from_native)
        }
    }

    /// Execute native `house_pos` using this complete owned configuration.
    ///
    /// See [`crate::global::house_pos`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `armc` - Right ascension of the Midheaven in degrees.
    /// * `latitude` - Geographic north latitude in degrees.
    /// * `obliquity` - Ecliptic obliquity in degrees.
    /// * `system` - ASCII house-system letter; fixed twelve-cusp operations reject Gauquelin.
    /// * `longitude` - Point longitude/right ascension in degrees.
    /// * `body_latitude` - Ecliptic latitude of the point in degrees.
    ///
    /// # Returns
    ///
    /// Complete HousePosition result, preserving actual producer channels.
    ///
    /// Sunshine (`I`/`i`) requires a solar declination absent from this request
    /// shape and returns `Unsupported` rather than inheriting native history.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn house_pos(
        &self,
        armc: f64,
        latitude: f64,
        obliquity: f64,
        system: u8,
        longitude: f64,
        body_latitude: f64,
    ) -> Result<HousePosition, Error> {
        // Sunshine's C point-position entry reads a solar declination retained
        // by an earlier houses call. This signature cannot supply that physical
        // input, so an owned primary call must not inherit the previous value.
        if system.eq_ignore_ascii_case(&b'I') {
            return Err(Error::unsupported(
                "house_pos",
                "Sunshine point positions require a prior native solar-declination context; use the explicit native SDK sequence",
            ));
        }

        let v = self
            .native
            .compute(|| {
                crate::houses::house_pos_locked(
                    armc,
                    latitude,
                    obliquity,
                    system,
                    longitude,
                    body_latitude,
                )
            })
            .map_err(Error::from_native)?;
        Ok(HousePosition {
            value: v.value,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `gauquelin_sector` using this complete owned configuration.
    ///
    /// See [`crate::global::gauquelin_sector`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `target` - Borrowed body identifier or fixed-star name.
    /// * `method` - Native Gauquelin method selector.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `pressure` - Atmospheric pressure in hPa; zero requests native estimation.
    /// * `temperature` - Air temperature in degrees Celsius.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete GauquelinSector result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn gauquelin_sector(
        &self,
        jd: f64,
        target: &Target,
        method: i32,
        site: [f64; 3],
        pressure: f64,
        temperature: f64,
        flags: i32,
    ) -> Result<GauquelinSector, Error> {
        let (body, star) = target_parts(target);
        let v = self
            .native
            .compute(|| {
                crate::houses::gauquelin_sector_complete_locked(
                    jd,
                    body,
                    star,
                    flags,
                    method,
                    site[0],
                    site[1],
                    site[2],
                    pressure,
                    temperature,
                )
            })
            .map_err(Error::from_native)?;
        Ok(GauquelinSector {
            value: v.0.value,
            status: Some(v.1),
            diagnostic: Some(v.0.diagnostic),
        })
    }

    /// Execute native `get_ayanamsa` using this complete owned configuration.
    ///
    /// See [`crate::global::get_ayanamsa`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    ///
    /// # Returns
    ///
    /// Complete Angle result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn get_ayanamsa(&self, jd: f64) -> Result<Angle, Error> {
        let v = self
            .native
            .compute(|| crate::config::get_ayanamsa_locked(jd))
            .map_err(Error::from_native)?;
        Ok(Angle { value: v })
    }

    /// Execute native `get_ayanamsa_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::get_ayanamsa_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    ///
    /// # Returns
    ///
    /// Complete Angle result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn get_ayanamsa_ut(&self, jd: f64) -> Result<Angle, Error> {
        let v = self
            .native
            .compute(|| crate::config::get_ayanamsa_ut_locked(jd))
            .map_err(Error::from_native)?;
        Ok(Angle { value: v })
    }

    /// Execute native `get_ayanamsa_ex` using this complete owned configuration.
    ///
    /// See [`crate::global::get_ayanamsa_ex`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Ayanamsha result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn get_ayanamsa_ex(&self, jd: f64, flags: i32) -> Result<Ayanamsha, Error> {
        let v = self
            .native
            .compute(|| crate::config::get_ayanamsa_ex_locked(jd, flags))
            .map_err(Error::from_native)?;
        Ok(Ayanamsha {
            value: v.value,
            returned_flags: v.returned_flags,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `get_ayanamsa_ex_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::get_ayanamsa_ex_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Ayanamsha result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn get_ayanamsa_ex_ut(&self, jd: f64, flags: i32) -> Result<Ayanamsha, Error> {
        let v = self
            .native
            .compute(|| crate::config::get_ayanamsa_ex_ut_locked(jd, flags))
            .map_err(Error::from_native)?;
        Ok(Ayanamsha {
            value: v.value,
            returned_flags: v.returned_flags,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `utc_to_jd` using this complete owned configuration.
    ///
    /// See [`crate::global::utc_to_jd`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `year` - Astronomical calendar year; zero denotes 1 BCE.
    /// * `month` - Civil month label; native validation/normalization is retained.
    /// * `day` - Civil day label; native validation/normalization is retained.
    /// * `hour` - Integer clock hour in the supplied UTC/local date.
    /// * `minute` - Clock minutes in the supplied local/UTC date.
    /// * `second` - Clock seconds, including a fractional part and admitted leap seconds.
    /// * `calendar` - Explicit proleptic Julian or Gregorian calendar.
    ///
    /// # Returns
    ///
    /// Complete JulDays result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn utc_to_jd(
        &self,
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: f64,
        calendar: Calendar,
    ) -> Result<JulDays, Error> {
        let v = self
            .native
            .compute(|| {
                crate::time::utc_to_jd_locked(
                    year,
                    month,
                    day,
                    hour,
                    minute,
                    second,
                    calendar.native(),
                )
            })
            .map_err(Error::from_native)?;
        Ok(JulDays {
            tt: v.jd_et,
            ut1: v.jd_ut,
        })
    }

    /// Execute native `jdet_to_utc` using this complete owned configuration.
    ///
    /// See [`crate::global::jdet_to_utc`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `calendar` - Explicit proleptic Julian or Gregorian calendar.
    ///
    /// # Returns
    ///
    /// Complete UtcDateTime result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn jdet_to_utc(&self, jd: f64, calendar: Calendar) -> Result<UtcDateTime, Error> {
        let v = self
            .native
            .compute(|| crate::time::jdet_to_utc_locked(jd, calendar.native()))
            .map_err(Error::from_native)?;
        Ok(super::pure::utc_date(v))
    }

    /// Execute native `jdut1_to_utc` using this complete owned configuration.
    ///
    /// See [`crate::global::jdut1_to_utc`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `calendar` - Explicit proleptic Julian or Gregorian calendar.
    ///
    /// # Returns
    ///
    /// Complete UtcDateTime result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn jdut1_to_utc(&self, jd: f64, calendar: Calendar) -> Result<UtcDateTime, Error> {
        let v = self
            .native
            .compute(|| crate::time::jdut1_to_utc_locked(jd, calendar.native()))
            .map_err(Error::from_native)?;
        Ok(super::pure::utc_date(v))
    }

    /// Execute native `deltat` using this complete owned configuration.
    ///
    /// See [`crate::global::deltat`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    ///
    /// # Returns
    ///
    /// Complete DeltaT result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn deltat(&self, jd: f64) -> Result<DeltaT, Error> {
        if !jd.is_finite() {
            return Err(Error::invalid("deltat", "Julian Day must be finite"));
        }
        let v = self
            .native
            .compute(|| Ok(crate::time::deltat_locked(jd)))
            .map_err(Error::from_native)?;
        Ok(DeltaT {
            days: v,
            diagnostic: None,
        })
    }

    /// Execute native `deltat_ex` using this complete owned configuration.
    ///
    /// See [`crate::global::deltat_ex`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete DeltaT result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn deltat_ex(&self, jd: f64, flags: i32) -> Result<DeltaT, Error> {
        let v = self
            .native
            .compute(|| crate::time::deltat_ex_locked(jd, flags))
            .map_err(Error::from_native)?;
        Ok(DeltaT {
            days: v.value,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `time_equ` using this complete owned configuration.
    ///
    /// See [`crate::global::time_equ`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    ///
    /// # Returns
    ///
    /// Complete EquationOfTime result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn time_equ(&self, jd: f64) -> Result<EquationOfTime, Error> {
        let v = self
            .native
            .compute(|| crate::time::time_equ_complete_locked(jd))
            .map_err(Error::from_native)?;
        Ok(EquationOfTime {
            days: v.0.value,
            status: Some(v.1),
            diagnostic: Some(v.0.diagnostic),
        })
    }

    /// Execute native `lat_to_lmt` using this complete owned configuration.
    ///
    /// See [`crate::global::lat_to_lmt`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Local apparent/mean Julian Day in days, as selected by the conversion.
    /// * `longitude` - Geographic east longitude in degrees.
    ///
    /// # Returns
    ///
    /// Complete LocalTime result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lat_to_lmt(&self, jd: f64, longitude: f64) -> Result<LocalTime, Error> {
        let v = self
            .native
            .compute(|| crate::time::lat_to_lmt_complete_locked(jd, longitude))
            .map_err(Error::from_native)?;
        Ok(LocalTime {
            julian_day: v.0.jd,
            status: Some(v.1),
            diagnostic: Some(v.0.diagnostic),
        })
    }

    /// Execute native `lmt_to_lat` using this complete owned configuration.
    ///
    /// See [`crate::global::lmt_to_lat`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Local apparent/mean Julian Day in days, as selected by the conversion.
    /// * `longitude` - Geographic east longitude in degrees.
    ///
    /// # Returns
    ///
    /// Complete LocalTime result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lmt_to_lat(&self, jd: f64, longitude: f64) -> Result<LocalTime, Error> {
        let v = self
            .native
            .compute(|| crate::time::lmt_to_lat_complete_locked(jd, longitude))
            .map_err(Error::from_native)?;
        Ok(LocalTime {
            julian_day: v.0.jd,
            status: Some(v.1),
            diagnostic: Some(v.0.diagnostic),
        })
    }

    /// Execute native `sidtime` using this complete owned configuration.
    ///
    /// See [`crate::global::sidtime`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    ///
    /// # Returns
    ///
    /// Complete SiderealTime result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn sidtime(&self, jd: f64) -> Result<SiderealTime, Error> {
        let v = self
            .native
            .compute(|| crate::time::sidtime_locked(jd))
            .map_err(Error::from_native)?;
        Ok(SiderealTime {
            hours: v,
            diagnostic: None,
        })
    }

    /// Execute native `sidtime0` using this complete owned configuration.
    ///
    /// See [`crate::global::sidtime0`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `obliquity` - Ecliptic obliquity in degrees.
    /// * `nutation` - Nutation in longitude in degrees.
    ///
    /// # Returns
    ///
    /// Complete SiderealTime result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn sidtime0(&self, jd: f64, obliquity: f64, nutation: f64) -> Result<SiderealTime, Error> {
        let v = self
            .native
            .compute(|| crate::time::sidtime0_locked(jd, obliquity, nutation))
            .map_err(Error::from_native)?;
        Ok(SiderealTime {
            hours: v,
            diagnostic: None,
        })
    }

    /// Execute native `solcross` using this complete owned configuration.
    ///
    /// See [`crate::global::solcross`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `longitude` - Target ecliptic longitude in degrees.
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete LongitudeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn solcross(
        &self,
        longitude: f64,
        jd: f64,
        flags: i32,
    ) -> Result<LongitudeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::solcross_locked(longitude, jd, flags))
            .map_err(Error::from_native)?;
        Ok(LongitudeCrossing {
            time: v.time,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `solcross_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::solcross_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `longitude` - Target ecliptic longitude in degrees.
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete LongitudeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn solcross_ut(
        &self,
        longitude: f64,
        jd: f64,
        flags: i32,
    ) -> Result<LongitudeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::solcross_ut_locked(longitude, jd, flags))
            .map_err(Error::from_native)?;
        Ok(LongitudeCrossing {
            time: v.time,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `mooncross` using this complete owned configuration.
    ///
    /// See [`crate::global::mooncross`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `longitude` - Target ecliptic longitude in degrees.
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete LongitudeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn mooncross(
        &self,
        longitude: f64,
        jd: f64,
        flags: i32,
    ) -> Result<LongitudeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::mooncross_locked(longitude, jd, flags))
            .map_err(Error::from_native)?;
        Ok(LongitudeCrossing {
            time: v.time,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `mooncross_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::mooncross_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `longitude` - Target ecliptic longitude in degrees.
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete LongitudeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn mooncross_ut(
        &self,
        longitude: f64,
        jd: f64,
        flags: i32,
    ) -> Result<LongitudeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::mooncross_ut_locked(longitude, jd, flags))
            .map_err(Error::from_native)?;
        Ok(LongitudeCrossing {
            time: v.time,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `mooncross_node` using this complete owned configuration.
    ///
    /// See [`crate::global::mooncross_node`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete NodeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn mooncross_node(&self, jd: f64, flags: i32) -> Result<NodeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::mooncross_node_locked(jd, flags))
            .map_err(Error::from_native)?;
        Ok(NodeCrossing {
            time: v.time,
            longitude: v.longitude,
            latitude: v.latitude,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `mooncross_node_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::mooncross_node_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete NodeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn mooncross_node_ut(&self, jd: f64, flags: i32) -> Result<NodeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::mooncross_node_ut_locked(jd, flags))
            .map_err(Error::from_native)?;
        Ok(NodeCrossing {
            time: v.time,
            longitude: v.longitude,
            latitude: v.latitude,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `helio_cross` using this complete owned configuration.
    ///
    /// See [`crate::global::helio_cross`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `longitude` - Target ecliptic longitude in degrees.
    /// * `jd` - Ephemeris/Terrestrial Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `direction` - Native heliocentric search direction (+1 forward, -1 backward).
    ///
    /// # Returns
    ///
    /// Complete LongitudeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn helio_cross(
        &self,
        body: i32,
        longitude: f64,
        jd: f64,
        flags: i32,
        direction: i32,
    ) -> Result<LongitudeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::helio_cross_locked(body, longitude, jd, flags, direction))
            .map_err(Error::from_native)?;
        Ok(LongitudeCrossing {
            time: v.time,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `helio_cross_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::helio_cross_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `body` - Native body identifier; data availability follows the selected provider.
    /// * `longitude` - Target ecliptic longitude in degrees.
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `direction` - Native heliocentric search direction (+1 forward, -1 backward).
    ///
    /// # Returns
    ///
    /// Complete LongitudeCrossing result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn helio_cross_ut(
        &self,
        body: i32,
        longitude: f64,
        jd: f64,
        flags: i32,
        direction: i32,
    ) -> Result<LongitudeCrossing, Error> {
        let v = self
            .native
            .compute(|| crate::events::helio_cross_ut_locked(body, longitude, jd, flags, direction))
            .map_err(Error::from_native)?;
        Ok(LongitudeCrossing {
            time: v.time,
            diagnostic: v.diagnostic,
        })
    }

    /// Execute native `rise_trans` using this complete owned configuration.
    ///
    /// See [`crate::global::rise_trans`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `target` - Borrowed body identifier or fixed-star name.
    /// * `event` - Native rise/set/transit event and horizon bitmask.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `pressure` - Atmospheric pressure in hPa; zero requests native estimation.
    /// * `temperature` - Air temperature in degrees Celsius.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete RiseTransit result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn rise_trans(
        &self,
        jd: f64,
        target: &Target,
        event: i32,
        site: [f64; 3],
        pressure: f64,
        temperature: f64,
        flags: i32,
    ) -> Result<RiseTransit, Error> {
        let (body, star) = target_parts(target);
        let v = self
            .native
            .compute(|| {
                crate::observer::rise_trans_complete_locked(
                    jd,
                    body,
                    star,
                    flags,
                    event,
                    site[0],
                    site[1],
                    site[2],
                    pressure,
                    temperature,
                )
            })
            .map_err(Error::from_native)?;
        Ok(rise(v))
    }

    /// Execute native `rise_trans_true_hor` using this complete owned configuration.
    ///
    /// See [`crate::global::rise_trans_true_hor`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `target` - Borrowed body identifier or fixed-star name.
    /// * `event` - Native rise/set/transit event and horizon bitmask.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `pressure` - Atmospheric pressure in hPa; zero requests native estimation.
    /// * `temperature` - Air temperature in degrees Celsius.
    /// * `horizon` - Apparent local horizon height in degrees.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete RiseTransit result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn rise_trans_true_hor(
        &self,
        jd: f64,
        target: &Target,
        event: i32,
        site: [f64; 3],
        pressure: f64,
        temperature: f64,
        horizon: f64,
        flags: i32,
    ) -> Result<RiseTransit, Error> {
        let (body, star) = target_parts(target);
        let v = self
            .native
            .compute(|| {
                crate::observer::rise_trans_true_hor_complete_locked(
                    jd,
                    body,
                    star,
                    flags,
                    event,
                    site[0],
                    site[1],
                    site[2],
                    pressure,
                    temperature,
                    horizon,
                )
            })
            .map_err(Error::from_native)?;
        Ok(rise(v))
    }

    /// Execute native `azalt` using this complete owned configuration.
    ///
    /// See [`crate::global::azalt`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flag` - Native horizontal-frame or refraction-direction selector.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `pressure` - Atmospheric pressure in hPa; zero requests native estimation.
    /// * `temperature` - Air temperature in degrees Celsius.
    /// * `longitude` - Point longitude/right ascension in degrees.
    /// * `latitude` - Point latitude/declination in degrees.
    ///
    /// # Returns
    ///
    /// Complete Horizontal result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn azalt(
        &self,
        jd: f64,
        flag: i32,
        site: [f64; 3],
        pressure: f64,
        temperature: f64,
        longitude: f64,
        latitude: f64,
    ) -> Result<Horizontal, Error> {
        let v = self
            .native
            .compute(|| {
                crate::observer::azalt_locked(
                    jd,
                    flag,
                    site[0],
                    site[1],
                    site[2],
                    pressure,
                    temperature,
                    longitude,
                    latitude,
                )
            })
            .map_err(Error::from_native)?;
        Ok(Horizontal {
            azimuth: v.azimuth,
            true_altitude: v.true_altitude,
            apparent_altitude: v.apparent_altitude,
        })
    }

    /// Execute native `azalt_rev` using this complete owned configuration.
    ///
    /// See [`crate::global::azalt_rev`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flag` - Native horizontal-frame or refraction-direction selector.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `azimuth` - Native azimuth in degrees: south zero, west positive.
    /// * `altitude` - True horizontal altitude in degrees.
    ///
    /// # Returns
    ///
    /// Complete Celestial result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn azalt_rev(
        &self,
        jd: f64,
        flag: i32,
        site: [f64; 3],
        azimuth: f64,
        altitude: f64,
    ) -> Result<Celestial, Error> {
        let v = self
            .native
            .compute(|| {
                crate::observer::azalt_rev_locked(
                    jd, flag, site[0], site[1], site[2], azimuth, altitude,
                )
            })
            .map_err(Error::from_native)?;
        Ok(Celestial {
            longitude_or_ra: v.longitude_or_ra,
            latitude_or_dec: v.latitude_or_dec,
        })
    }

    /// Execute native `refrac` using this complete owned configuration.
    ///
    /// See [`crate::global::refrac`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `altitude` - True/apparent altitude in degrees, selected by the conversion flag.
    /// * `pressure` - Atmospheric pressure in hPa; zero requests native estimation.
    /// * `temperature` - Air temperature in degrees Celsius.
    /// * `flag` - Native horizontal-frame or refraction-direction selector.
    ///
    /// # Returns
    ///
    /// Complete Angle result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn refrac(
        &self,
        altitude: f64,
        pressure: f64,
        temperature: f64,
        flag: i32,
    ) -> Result<Angle, Error> {
        let v = self
            .native
            .compute(|| crate::observer::refrac_locked(altitude, pressure, temperature, flag))
            .map_err(Error::from_native)?;
        Ok(Angle { value: v })
    }

    /// Execute native `refrac_extended` using this complete owned configuration.
    ///
    /// See [`crate::global::refrac_extended`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `altitude` - True/apparent altitude in degrees, selected by the conversion flag.
    /// * `height` - Observer height above sea level in metres.
    /// * `pressure` - Atmospheric pressure in hPa; zero requests native estimation.
    /// * `temperature` - Air temperature in degrees Celsius.
    /// * `lapse_rate` - Temperature lapse rate in kelvin per metre.
    /// * `flag` - Native horizontal-frame or refraction-direction selector.
    ///
    /// # Returns
    ///
    /// Complete Refraction result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn refrac_extended(
        &self,
        altitude: f64,
        height: f64,
        pressure: f64,
        temperature: f64,
        lapse_rate: f64,
        flag: i32,
    ) -> Result<Refraction, Error> {
        let v = self
            .native
            .compute(|| {
                crate::observer::refrac_extended_locked(
                    altitude,
                    height,
                    pressure,
                    temperature,
                    lapse_rate,
                    flag,
                )
            })
            .map_err(Error::from_native)?;
        Ok(Refraction {
            value: v.converted,
            details: [v.true_altitude, v.apparent_altitude, v.refraction, v.dip],
            diagnostic: None,
        })
    }

    /// Execute native `sol_eclipse_when_glob` using this complete owned configuration.
    ///
    /// See [`crate::global::sol_eclipse_when_glob`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `eclipse_type` - Native eclipse-type filter bitmask; zero accepts all types.
    /// * `backwards` - Native backward-search choice.
    ///
    /// # Returns
    ///
    /// Complete GlobalSolarEclipse result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn sol_eclipse_when_glob(
        &self,
        jd: f64,
        flags: i32,
        eclipse_type: i32,
        backwards: bool,
    ) -> Result<GlobalSolarEclipse, Error> {
        let v = self
            .native
            .compute(|| {
                crate::events::sol_eclipse_when_glob_locked(jd, flags, eclipse_type, backwards)
            })
            .map_err(Error::from_native)?;
        Ok(GlobalSolarEclipse {
            status: v.eclipse_type,
            times: v.times,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `lun_eclipse_when` using this complete owned configuration.
    ///
    /// See [`crate::global::lun_eclipse_when`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `eclipse_type` - Native eclipse-type filter bitmask; zero accepts all types.
    /// * `backwards` - Native backward-search choice.
    ///
    /// # Returns
    ///
    /// Complete GlobalLunarEclipse result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lun_eclipse_when(
        &self,
        jd: f64,
        flags: i32,
        eclipse_type: i32,
        backwards: bool,
    ) -> Result<GlobalLunarEclipse, Error> {
        let v = self
            .native
            .compute(|| crate::events::lun_eclipse_when_locked(jd, flags, eclipse_type, backwards))
            .map_err(Error::from_native)?;
        Ok(GlobalLunarEclipse {
            status: v.eclipse_type,
            times: v.times,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `sol_eclipse_where` using this complete owned configuration.
    ///
    /// The full native transport is retained; primary records carry every declared slot.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete SolarEclipseGeometry result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn sol_eclipse_where(&self, jd: f64, flags: i32) -> Result<SolarEclipseGeometry, Error> {
        let v = self
            .native
            .compute(|| crate::events::sol_eclipse_where_full_locked(jd, flags))
            .map_err(Error::from_native)?;
        Ok(SolarEclipseGeometry {
            status: v.eclipse_type,
            geography: v.geopos,
            attributes: v.attributes,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `sol_eclipse_how` using this complete owned configuration.
    ///
    /// See [`crate::global::sol_eclipse_how`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete SolarEclipseCircumstances result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn sol_eclipse_how(
        &self,
        jd: f64,
        site: [f64; 3],
        flags: i32,
    ) -> Result<SolarEclipseCircumstances, Error> {
        let v = self
            .native
            .compute(|| crate::events::sol_eclipse_how_locked(jd, flags, site[0], site[1], site[2]))
            .map_err(Error::from_native)?;
        Ok(SolarEclipseCircumstances {
            status: v.eclipse_type,
            attributes: v.attributes,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `lun_eclipse_how` using this complete owned configuration.
    ///
    /// See [`crate::global::lun_eclipse_how`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete LunarEclipseCircumstances result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lun_eclipse_how(
        &self,
        jd: f64,
        site: [f64; 3],
        flags: i32,
    ) -> Result<LunarEclipseCircumstances, Error> {
        let v = self
            .native
            .compute(|| {
                crate::events::lun_eclipse_how_locked(jd, flags, Some((site[0], site[1], site[2])))
            })
            .map_err(Error::from_native)?;
        Ok(LunarEclipseCircumstances {
            status: v.eclipse_type,
            attributes: v.attributes,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `sol_eclipse_when_loc` using this complete owned configuration.
    ///
    /// See [`crate::global::sol_eclipse_when_loc`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `backwards` - Native backward-search choice.
    ///
    /// # Returns
    ///
    /// Complete LocalSolarEclipse result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn sol_eclipse_when_loc(
        &self,
        jd: f64,
        site: [f64; 3],
        flags: i32,
        backwards: bool,
    ) -> Result<LocalSolarEclipse, Error> {
        let v = self
            .native
            .compute(|| {
                crate::events::sol_eclipse_when_loc_locked(
                    jd, flags, site[0], site[1], site[2], backwards,
                )
            })
            .map_err(Error::from_native)?;
        Ok(LocalSolarEclipse {
            status: v.eclipse_type,
            maximum: Some(v.times[0]),
            first_contact: Some(v.times[1]),
            second_contact: Some(v.times[2]),
            third_contact: Some(v.times[3]),
            fourth_contact: Some(v.times[4]),
            sunrise: Some(v.times[5]),
            sunset: Some(v.times[6]),
            times: LocalSolarTimes::Seven(v.times),
            attributes: v.attributes,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `lun_eclipse_when_loc` using this complete owned configuration.
    ///
    /// See [`crate::global::lun_eclipse_when_loc`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `backwards` - Native backward-search choice.
    ///
    /// # Returns
    ///
    /// Complete LocalLunarEclipse result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lun_eclipse_when_loc(
        &self,
        jd: f64,
        site: [f64; 3],
        flags: i32,
        backwards: bool,
    ) -> Result<LocalLunarEclipse, Error> {
        let v = self
            .native
            .compute(|| {
                crate::events::lun_eclipse_when_loc_locked(
                    jd, flags, site[0], site[1], site[2], backwards,
                )
            })
            .map_err(Error::from_native)?;
        Ok(LocalLunarEclipse {
            status: v.eclipse_type,
            times: v.times,
            attributes: v.attributes,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `lun_occult_where` using this complete owned configuration.
    ///
    /// The full native transport is retained; primary records carry every declared slot.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `target` - Borrowed body identifier or fixed-star name.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete OccultationGeometry result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lun_occult_where(
        &self,
        jd: f64,
        target: &Target,
        flags: i32,
    ) -> Result<OccultationGeometry, Error> {
        let (body, star) = target_parts(target);
        let v = self
            .native
            .compute(|| crate::events::lun_occult_where_full_locked(jd, body, star, flags))
            .map_err(Error::from_native)?;
        Ok(OccultationGeometry {
            status: v.occult_type,
            geography: v.geopos,
            attributes: v.attributes,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `lun_occult_when_glob` using this complete owned configuration.
    ///
    /// See [`crate::global::lun_occult_when_glob`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `target` - Borrowed body identifier or fixed-star name.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `eclipse_type` - Native eclipse-type filter bitmask; zero accepts all types.
    /// * `backwards` - Direction bit and optional ECL_ONE_TRY; other bits are rejected.
    ///
    /// # Returns
    ///
    /// Complete GlobalOccultation result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lun_occult_when_glob(
        &self,
        jd: f64,
        target: &Target,
        flags: i32,
        eclipse_type: i32,
        backwards: i32,
    ) -> Result<GlobalOccultation, Error> {
        if backwards & !(1 | crate::constants::ECL_ONE_TRY) != 0 {
            return Err(Error::invalid(
                "lun_occult_when_glob",
                "backwards accepts only direction and ECL_ONE_TRY bits",
            ));
        }
        let (body, star) = target_parts(target);
        let v = self
            .native
            .compute(|| {
                crate::events::lun_occult_when_glob_impl_locked(
                    jd,
                    body,
                    star,
                    flags,
                    eclipse_type,
                    crate::events::OccultSearchOptions {
                        backward: backwards & 1 != 0,
                        one_try: backwards & crate::constants::ECL_ONE_TRY != 0,
                    },
                )
            })
            .map_err(Error::from_native)?;
        Ok(GlobalOccultation {
            status: v.occult_type,
            times: v.times,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `lun_occult_when_loc` using this complete owned configuration.
    ///
    /// See [`crate::global::lun_occult_when_loc`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `target` - Borrowed body identifier or fixed-star name.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    /// * `backwards` - Direction bit and optional ECL_ONE_TRY; other bits are rejected.
    ///
    /// # Returns
    ///
    /// Complete LocalOccultation result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn lun_occult_when_loc(
        &self,
        jd: f64,
        target: &Target,
        site: [f64; 3],
        flags: i32,
        backwards: i32,
    ) -> Result<LocalOccultation, Error> {
        if backwards & !(1 | crate::constants::ECL_ONE_TRY) != 0 {
            return Err(Error::invalid(
                "lun_occult_when_loc",
                "backwards accepts only direction and ECL_ONE_TRY bits",
            ));
        }
        let (body, star) = target_parts(target);
        let v = self
            .native
            .compute(|| {
                crate::events::lun_occult_when_loc_impl_locked(
                    jd,
                    body,
                    star,
                    flags,
                    site[0],
                    site[1],
                    site[2],
                    crate::events::OccultSearchOptions {
                        backward: backwards & 1 != 0,
                        one_try: backwards & crate::constants::ECL_ONE_TRY != 0,
                    },
                )
            })
            .map_err(Error::from_native)?;
        Ok(LocalOccultation {
            status: v.occult_type,
            times: v.times,
            attributes: v.attributes,
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `heliacal_ut` using this complete owned configuration.
    ///
    /// See [`crate::global::heliacal_ut`] for native units, frame and admission limits.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `atmosphere` - Pressure hPa, temperature Celsius, humidity percent, and native extinction/visibility selector.
    /// * `observer` - Six native observer/optical inputs, preserving native zero-default semantics.
    /// * `object` - Native planet or fixed-star query for heliacal visibility.
    /// * `event` - Native heliacal event selector.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete HeliacalEvent result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn heliacal_ut(
        &self,
        jd: f64,
        site: [f64; 3],
        atmosphere: [f64; 4],
        observer: [f64; 6],
        object: &str,
        event: i32,
        flags: i32,
    ) -> Result<HeliacalEvent, Error> {
        let v = self
            .native
            .compute(|| {
                crate::events::heliacal_ut_complete_locked(
                    jd, site[0], site[1], site[2], atmosphere, observer, object, event, flags,
                )
            })
            .map_err(Error::from_native)?;
        Ok(HeliacalEvent {
            values: v.0.times.to_vec(),
            status: Some(v.1),
            diagnostic: Some(v.0.diagnostic),
        })
    }

    /// Execute native `heliacal_pheno_ut` using this complete owned configuration.
    ///
    /// The full native transport is retained; primary records carry every declared slot.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `atmosphere` - Pressure hPa, temperature Celsius, humidity percent, and native extinction/visibility selector.
    /// * `observer` - Six native observer/optical inputs, preserving native zero-default semantics.
    /// * `object` - Native planet or fixed-star query for heliacal visibility.
    /// * `event` - Native heliacal event selector.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete HeliacalPhenomena result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn heliacal_pheno_ut(
        &self,
        jd: f64,
        site: [f64; 3],
        atmosphere: [f64; 4],
        observer: [f64; 6],
        object: &str,
        event: i32,
        flags: i32,
    ) -> Result<HeliacalPhenomena, Error> {
        let v = self
            .native
            .compute(|| {
                crate::events::heliacal_pheno_ut_full_locked(
                    jd, site[0], site[1], site[2], atmosphere, observer, object, event, flags,
                )
            })
            .map_err(Error::from_native)?;
        Ok(HeliacalPhenomena {
            values: v.values,
            status: Some(v.status),
            diagnostic: Some(v.diagnostic),
        })
    }

    /// Execute native `vis_limit_mag` using this complete owned configuration.
    ///
    /// The full native transport is retained; primary records carry every declared slot.
    ///
    /// # Arguments
    ///
    /// * `jd` - Universal Time Julian Day in days.
    /// * `site` - East longitude/north latitude in degrees, then observer height in metres.
    /// * `atmosphere` - Pressure hPa, temperature Celsius, humidity percent, and native extinction/visibility selector.
    /// * `observer` - Six native observer/optical inputs, preserving native zero-default semantics.
    /// * `object` - Native planet or fixed-star query for heliacal visibility.
    /// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
    ///
    /// # Returns
    ///
    /// Complete Visibility result, preserving actual producer channels.
    ///
    /// # Errors
    ///
    /// Original native input, data, calculation or locking failures propagate.
    /// External lookup context changes require rebuilding the session.
    #[allow(clippy::too_many_arguments)]
    pub fn vis_limit_mag(
        &self,
        jd: f64,
        site: [f64; 3],
        atmosphere: [f64; 4],
        observer: [f64; 6],
        object: &str,
        flags: i32,
    ) -> Result<Visibility, Error> {
        let v = self
            .native
            .compute(|| {
                crate::events::vis_limit_mag_full_locked(
                    jd, site[0], site[1], site[2], atmosphere, observer, object, flags,
                )
            })
            .map_err(Error::from_native)?;
        Ok(Visibility {
            scalar: None,
            status: Some(v.status),
            values: v.values,
            diagnostic: Some(v.diagnostic),
        })
    }
}

/// Build a primary session from explicit engine-specific SDK data settings.
/// All otherwise omitted settings are completed with owned defaults.
pub(crate) fn from_native_builder(
    builder: &crate::session::SessionBuilder,
) -> Result<Session, Error> {
    Ok(Session {
        native: builder.build_owned().map_err(Error::from_native)?,
    })
}
fn target_parts(target: &Target) -> (i32, Option<&str>) {
    match target {
        Target::Body(body) => (*body, None),
        Target::Star(name) => (0, Some(name)),
    }
}
fn position(v: crate::positions::Position) -> Position {
    Position {
        values: v.values,
        returned_flags: v.returned_flags,
        diagnostic: v.diagnostic,
        warnings: Vec::new(),
        messages: Vec::new(),
    }
}
fn rise((v, status): (crate::observer::RiseTransitOutcome, i32)) -> RiseTransit {
    match v {
        crate::observer::RiseTransitOutcome::Event {
            time_ut,
            diagnostic,
        } => RiseTransit {
            status,
            time: Some(time_ut),
            times: RiseTransitTimes::One([time_ut]),
            diagnostic: Some(diagnostic),
        },
        crate::observer::RiseTransitOutcome::Circumpolar { diagnostic } => RiseTransit {
            status,
            time: None,
            times: RiseTransitTimes::Absent,
            diagnostic: Some(diagnostic),
        },
    }
}

/// Read the retained native snapshot without installing configuration.
pub(crate) fn current_file_data(
    session: &Session,
    slot: i32,
) -> Result<Option<crate::native::CurrentFileData>, crate::native::Error> {
    session.native.get_current_file_data(slot)
}

/// Reuse caller-owned path storage while reading the retained native snapshot.
pub(crate) fn copy_file_path_into(
    session: &Session,
    slot: i32,
    path: &mut Option<String>,
) -> Result<(), crate::native::Error> {
    session.native.copy_captured_file_path_into(slot, path)
}
