//! Input scalar structs for `pipeline::compute_timestep`.
//!
//! `WeatherScalars`, `HumanScalars`, and `ConfigScalars` are constructed in
//! Python via PyO3 `#[new]` constructors and passed by reference to
//! `compute_timestep`. Extracted here so `pipeline.rs` can focus on the
//! per-timestep orchestration.


/// Weather scalars for a single timestep.
#[derive(Clone)]
pub struct WeatherScalars {
    pub sun_azimuth: f32,
    pub sun_altitude: f32,
    pub sun_zenith: f32,
    pub ta: f32,
    pub rh: f32,
    pub global_rad: f32,
    pub direct_rad: f32,
    pub diffuse_rad: f32,
    pub altmax: f32,
    pub clearness_index: f32,
    pub dectime: f32,
    pub snup: f32,
    pub rad_g0: f32,
    pub zen_deg: f32,
    pub is_daytime: bool,
    pub jday: i32,
    pub patch_option: i32,
}

/// Human body parameters.
#[derive(Clone)]
pub struct HumanScalars {
    /// Height the ground view factor is evaluated at (m): the posture's
    /// centre of gravity (UMEP `Posture.*.height`, 1.1 standing, 0.75
    /// sitting), not the stature.
    pub height: f32,
    pub abs_k: f32,
    pub abs_l: f32,
    pub is_standing: bool,
}

/// Configuration scalars (constant across timesteps).
#[derive(Clone)]
pub struct ConfigScalars {
    pub pixel_size: f32,
    pub max_height: f32,
    pub albedo_wall: f32,
    pub emis_wall: f32,
    pub tgk_wall: f32,
    pub tstart_wall: f32,
    pub tmaxlst_wall: f32,
    pub use_veg: bool,
    pub has_walls: bool,
    pub use_anisotropic: bool,
    pub max_shadow_distance_m: f32,
}
