# 1.x function path migration

Extension signatures retain the 1.x parameter types, C ordering and scalar
arguments. Primary Session operations use the separately specified ordering
and complete records. Raw pure-return helpers remain native extensions when
the primary pure operation adopts a typed result.

All 111 extension destinations below now exist as direct re-exports, without
wrappers or reordered arguments. The frozen-signature test names each exact
extension path explicitly. Configured free functions and native-only helpers
are no longer exported at the root. Root pure utilities remain the primary
route and are being converted to their final typed results.

| Old root path | Extension destination |
|---|---|
| `cotrans` | `native::cotrans` |
| `cotrans_sp` | `native::cotrans_sp` |
| `degnorm` | `native::degnorm` |
| `radnorm` | `native::radnorm` |
| `difdegn` | `native::difdegn` |
| `difdeg2n` | `native::difdeg2n` |
| `difrad2n` | `native::difrad2n` |
| `deg_midp` | `native::deg_midp` |
| `rad_midp` | `native::rad_midp` |
| `d2l` | `native::d2l` |
| `csnorm` | `native::csnorm` |
| `difcsn` | `native::difcsn` |
| `difcs2n` | `native::difcs2n` |
| `csroundsec` | `native::csroundsec` |
| `cs2timestr` | `native::cs2timestr` |
| `cs2lonlatstr` | `native::cs2lonlatstr` |
| `cs2degstr` | `native::cs2degstr` |
| `split_deg` | `native::split_deg` |
| `set_ephe_path` | `global::set_ephe_path` |
| `ensure_ephe_path` | `native::ensure_ephe_path` |
| `set_topo` | `global::set_topo` |
| `set_sid_mode` | `global::set_sid_mode` |
| `set_jpl_file` | `native::set_jpl_file` |
| `set_tid_acc` | `global::set_tid_acc` |
| `get_tid_acc` | `global::get_tid_acc` |
| `set_delta_t_userdef` | `global::set_delta_t_userdef` |
| `get_ayanamsa_ut` | `global::get_ayanamsa_ut` |
| `get_ayanamsa` | `global::get_ayanamsa` |
| `get_ayanamsa_ex` | `global::get_ayanamsa_ex` |
| `get_ayanamsa_ex_ut` | `global::get_ayanamsa_ex_ut` |
| `get_ayanamsa_name` | `native::get_ayanamsa_name` |
| `close` | `global::close` |
| `get_current_file_data` | `global::get_current_file_data` |
| `solcross` | `global::solcross` |
| `solcross_ut` | `global::solcross_ut` |
| `mooncross` | `global::mooncross` |
| `mooncross_ut` | `global::mooncross_ut` |
| `mooncross_node` | `global::mooncross_node` |
| `mooncross_node_ut` | `global::mooncross_node_ut` |
| `helio_cross` | `global::helio_cross` |
| `helio_cross_ut` | `global::helio_cross_ut` |
| `sol_eclipse_when_glob` | `global::sol_eclipse_when_glob` |
| `lun_eclipse_when` | `global::lun_eclipse_when` |
| `sol_eclipse_where_full` | `native::sol_eclipse_where_full` |
| `sol_eclipse_where` | `global::sol_eclipse_where` |
| `sol_eclipse_how` | `global::sol_eclipse_how` |
| `lun_eclipse_how` | `global::lun_eclipse_how` |
| `sol_eclipse_when_loc` | `global::sol_eclipse_when_loc` |
| `lun_eclipse_when_loc` | `global::lun_eclipse_when_loc` |
| `lun_occult_when_glob` | `global::lun_occult_when_glob` |
| `lun_occult_when_glob_with_options` | `native::lun_occult_when_glob_with_options` |
| `lun_occult_when_loc` | `global::lun_occult_when_loc` |
| `lun_occult_when_loc_with_options` | `native::lun_occult_when_loc_with_options` |
| `lun_occult_where_full` | `native::lun_occult_where_full` |
| `lun_occult_where` | `global::lun_occult_where` |
| `heliacal_ut` | `global::heliacal_ut` |
| `heliacal_pheno_ut_full` | `native::heliacal_pheno_ut_full` |
| `heliacal_pheno_ut` | `global::heliacal_pheno_ut` |
| `vis_limit_mag` | `global::vis_limit_mag` |
| `vis_limit_mag_full` | `native::vis_limit_mag_full` |
| `houses` | `global::houses` |
| `houses_ex` | `global::houses_ex` |
| `houses_ex2` | `global::houses_ex2` |
| `houses_gauquelin` | `global::houses_gauquelin` |
| `houses_armc` | `global::houses_armc` |
| `houses_armc_ex2` | `global::houses_armc_ex2` |
| `house_pos` | `global::house_pos` |
| `house_name` | `native::house_name` |
| `gauquelin_sector` | `global::gauquelin_sector` |
| `nod_aps` | `global::nod_aps` |
| `nod_aps_ut` | `global::nod_aps_ut` |
| `rise_trans` | `global::rise_trans` |
| `rise_trans_true_hor` | `global::rise_trans_true_hor` |
| `azalt` | `global::azalt` |
| `azalt_rev` | `global::azalt_rev` |
| `refrac` | `global::refrac` |
| `refrac_extended` | `global::refrac_extended` |
| `set_lapse_rate` | `global::set_lapse_rate` |
| `pheno` | `global::pheno` |
| `pheno_ut` | `global::pheno_ut` |
| `get_orbital_elements` | `global::get_orbital_elements` |
| `orbit_max_min_true_distance` | `global::orbit_max_min_true_distance` |
| `calc_ut` | `global::calc_ut` |
| `calc` | `global::calc` |
| `calc_pctr` | `global::calc_pctr` |
| `get_planet_name` | `global::get_planet_name` |
| `require_source_flags` | `native::require_source_flags` |
| `fixstar` | `global::fixstar` |
| `fixstar_ut` | `global::fixstar_ut` |
| `fixstar2` | `global::fixstar2` |
| `fixstar2_ut` | `global::fixstar2_ut` |
| `fixstar_mag` | `global::fixstar_mag` |
| `fixstar2_mag` | `global::fixstar2_mag` |
| `fixed_star_catalog` | `native::fixed_star_catalog` |
| `julday` | `native::julday` |
| `revjul` | `native::revjul` |
| `utc_to_jd` | `global::utc_to_jd` |
| `jdet_to_utc` | `global::jdet_to_utc` |
| `jdut1_to_utc` | `global::jdut1_to_utc` |
| `utc_time_zone` | `native::utc_time_zone` |
| `date_conversion` | `native::date_conversion` |
| `day_of_week` | `native::day_of_week` |
| `deltat` | `global::deltat` |
| `deltat_ex` | `global::deltat_ex` |
| `time_equ` | `global::time_equ` |
| `lmt_to_lat` | `global::lmt_to_lat` |
| `lat_to_lmt` | `global::lat_to_lmt` |
| `sidtime` | `global::sidtime` |
| `sidtime0` | `global::sidtime0` |
| `version` | `native::version` |
| `library_path` | `native::library_path` |
