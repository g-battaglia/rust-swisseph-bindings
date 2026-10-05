//! Public event fields and accessors, independent of C slot projectors.
//! Arrays are copied whole before appending each separately called getter.
use super::protocol::Record;
use swisseph_bindings::*;

/// Pack native type/status, all declared floating outputs and complete text.
fn event(code: i32, fields: impl IntoIterator<Item = f64>, diagnostic: String) -> Record {
    Record {
        code,
        floats: fields.into_iter().collect(),
        texts: vec![diagnostic.into_bytes()],
        ..Record::default()
    }
}
/// Ten global solar contacts plus the seven named contact accessors.
pub(super) fn solar_global(v: GlobalSolarEclipse) -> Record {
    event(
        v.eclipse_type,
        v.times.into_iter().chain([
            v.maximum(),
            v.begin(),
            v.end(),
            v.totality_begin(),
            v.totality_end(),
            v.centerline_begin(),
            v.centerline_end(),
        ]),
        v.diagnostic,
    )
}
/// Ten global lunar contacts plus their seven named accessors.
pub(super) fn lunar_global(v: GlobalLunarEclipse) -> Record {
    event(
        v.eclipse_type,
        v.times.into_iter().chain([
            v.maximum(),
            v.partial_begin(),
            v.partial_end(),
            v.total_begin(),
            v.total_end(),
            v.penumbral_begin(),
            v.penumbral_end(),
        ]),
        v.diagnostic,
    )
}
/// Geography followed by the entire twenty-slot attribute array.
pub(super) fn solar_geometry(v: SolarEclipseGeometry) -> Record {
    event(
        v.eclipse_type,
        [v.longitude, v.latitude].into_iter().chain(v.attributes),
        v.diagnostic,
    )
}
/// Solar attributes and each of their ten public getters.
pub(super) fn solar_how(v: SolarEclipseCircumstances) -> Record {
    event(
        v.eclipse_type,
        v.attributes.into_iter().chain([
            v.magnitude(),
            v.diameter_ratio(),
            v.obscuration(),
            v.core_shadow_km(),
            v.azimuth(),
            v.true_altitude(),
            v.apparent_altitude(),
            v.separation(),
            v.saros_series(),
            v.saros_member(),
        ]),
        v.diagnostic,
    )
}
/// Lunar attributes and all eight named getters, including observer values.
pub(super) fn lunar_how(v: LunarEclipseCircumstances) -> Record {
    event(
        v.eclipse_type,
        v.attributes.into_iter().chain([
            v.umbral_magnitude(),
            v.penumbral_magnitude(),
            v.azimuth(),
            v.true_altitude(),
            v.apparent_altitude(),
            v.opposition_distance(),
            v.saros_series(),
            v.saros_member(),
        ]),
        v.diagnostic,
    )
}
/// Seven solar contacts, twenty attributes and their seventeen named getters.
pub(super) fn solar_local(v: LocalSolarEclipse) -> Record {
    event(
        v.eclipse_type,
        v.times.into_iter().chain(v.attributes).chain([
            v.maximum(),
            v.first_contact(),
            v.second_contact(),
            v.third_contact(),
            v.fourth_contact(),
            v.sunrise(),
            v.sunset(),
            v.magnitude(),
            v.diameter_ratio(),
            v.obscuration(),
            v.core_shadow_km(),
            v.azimuth(),
            v.true_altitude(),
            v.apparent_altitude(),
            v.separation(),
            v.saros_series(),
            v.saros_member(),
        ]),
        v.diagnostic,
    )
}
/// Ten lunar contacts, twenty attributes and all seventeen named getters.
pub(super) fn lunar_local(v: LocalLunarEclipse) -> Record {
    event(
        v.eclipse_type,
        v.times.into_iter().chain(v.attributes).chain([
            v.maximum(),
            v.partial_begin(),
            v.partial_end(),
            v.total_begin(),
            v.total_end(),
            v.penumbral_begin(),
            v.penumbral_end(),
            v.moonrise(),
            v.moonset(),
            v.umbral_magnitude(),
            v.penumbral_magnitude(),
            v.azimuth(),
            v.true_altitude(),
            v.apparent_altitude(),
            v.opposition_distance(),
            v.saros_series(),
            v.saros_member(),
        ]),
        v.diagnostic,
    )
}
/// Global occultation contacts and all seven named contact getters.
pub(super) fn occult_global(v: GlobalOccultation) -> Record {
    event(
        v.occult_type,
        v.times.into_iter().chain([
            v.maximum(),
            v.begin(),
            v.end(),
            v.totality_begin(),
            v.totality_end(),
            v.centerline_begin(),
            v.centerline_end(),
        ]),
        v.diagnostic,
    )
}
/// Local occultation contacts/attributes and all fifteen named getters.
pub(super) fn occult_local(v: LocalOccultation) -> Record {
    event(
        v.occult_type,
        v.times.into_iter().chain(v.attributes).chain([
            v.maximum(),
            v.first_contact(),
            v.second_contact(),
            v.third_contact(),
            v.fourth_contact(),
            v.body_rise(),
            v.body_set(),
            v.magnitude(),
            v.diameter_ratio(),
            v.obscuration(),
            v.core_shadow_km(),
            v.azimuth(),
            v.true_altitude(),
            v.apparent_altitude(),
            v.separation(),
        ]),
        v.diagnostic,
    )
}
/// Occultation geography, all attributes and all eight attribute getters.
pub(super) fn occult_geometry(v: OccultationGeometry) -> Record {
    event(
        v.occult_type,
        [v.longitude, v.latitude]
            .into_iter()
            .chain(v.attributes)
            .chain([
                v.magnitude(),
                v.diameter_ratio(),
                v.obscuration(),
                v.core_shadow_km(),
                v.azimuth(),
                v.true_altitude(),
                v.apparent_altitude(),
                v.separation(),
            ]),
        v.diagnostic,
    )
}
