use ndarray::{Array2, ArrayView2, Zip};
use rayon::prelude::*;

/// Ground temperature calculation parameters
const PI: f32 = std::f32::consts::PI;

/// Pure result type for ground temperature (no PyO3 dependency).
pub(crate) struct GroundTempResult {
    pub tg: Array2<f32>,
    pub tg_wall: f32,
    pub ci_tg: f32,
}

/// Pure-ndarray implementation of ground temperature calculation.
/// Callable from pipeline.rs (fused path) or from the PyO3 wrapper (modular path).
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_ground_temperature_pure(
    sun_altitude: f32,
    altmax: f32,
    dectime: f32,
    snup: f32,
    global_rad: f32,
    rad_g0: f32,
    zen_deg: f32,
    tgk_grid: ArrayView2<f32>,
    tstart_grid: ArrayView2<f32>,
    tmaxlst_grid: ArrayView2<f32>,
    tgk_wall: f32,
    tstart_wall: f32,
    tmaxlst_wall: f32,
) -> GroundTempResult {
    let shape = tgk_grid.dim();

    // Temperature amplitude based on max sun altitude (per-pixel from land cover)
    let tgamp = &tgk_grid * altmax + &tstart_grid;

    // Wall temperature amplitude
    let tgamp_wall = tgk_wall * altmax + tstart_wall;

    // Phase calculation for ground (per-pixel)
    let snup_frac = snup / 24.0;
    let tmaxlst_frac = &tmaxlst_grid / 24.0;
    let tmaxlst_wall_frac = tmaxlst_wall / 24.0;

    let mut tg = Array2::<f32>::zeros(shape);

    // tg was just allocated via Array2::zeros so as_slice_mut() is Some.
    tg.as_slice_mut()
        .expect("tg: freshly-allocated Array2::zeros is contiguous")
        .par_iter_mut()
        .enumerate()
        .for_each(|(idx, out)| {
            let row = idx / shape.1;
            let col = idx % shape.1;

            let tgamp_val = tgamp[[row, col]];
            if !tgamp_val.is_finite() {
                *out = f32::NAN;
                return;
            }
            let tmaxlst_frac_val = tmaxlst_frac[[row, col]];

            if dectime > snup_frac {
                let denom = tmaxlst_frac_val - snup_frac;
                let denom = if denom > 0.0 { denom } else { 1.0 };
                let phase = (dectime - snup_frac) / denom;
                *out = tgamp_val * (phase * PI / 2.0).sin();
            } else {
                *out = 0.0;
            }
        });

    // Wall phase (scalar)
    let tg_wall = if dectime > snup_frac && tmaxlst_wall_frac > snup_frac {
        let denom_wall = tmaxlst_wall_frac - snup_frac;
        let denom_wall = if denom_wall > 0.0 { denom_wall } else { 1.0 };
        let phase_wall = (dectime - snup_frac) / denom_wall;
        tgamp_wall * (phase_wall * PI / 2.0).sin()
    } else {
        0.0
    };

    // CI_TgG correction for non-clear conditions
    let ci_tg = if sun_altitude > 0.0 && rad_g0 > 0.0 {
        let corr = if zen_deg > 0.0 && zen_deg < 90.0 {
            0.1473 * (90.0 - zen_deg).ln() + 0.3454
        } else {
            0.3454
        };
        let mut ci = (global_rad / rad_g0) + (1.0 - corr);
        ci = ci.min(1.0);
        if ci.is_infinite() || ci.is_nan() {
            1.0
        } else {
            ci
        }
    } else {
        1.0
    };

    // Apply clearness correction
    tg.par_mapv_inplace(|v| (v * ci_tg).max(0.0));
    let tg_wall_corrected = (tg_wall * ci_tg).max(0.0);

    GroundTempResult {
        tg,
        tg_wall: tg_wall_corrected,
        ci_tg,
    }
}

/// Pure result type for batched thermal delay (no PyO3 dependency).
pub(crate) struct TsWaveDelayBatchPureResult {
    pub lup: Array2<f32>,
    pub lup_e: Array2<f32>,
    pub lup_s: Array2<f32>,
    pub lup_w: Array2<f32>,
    pub lup_n: Array2<f32>,
    pub tg_out: Array2<f32>,
    pub timeadd: f32,
    pub tgmap1: Array2<f32>,
    pub tgmap1_e: Array2<f32>,
    pub tgmap1_s: Array2<f32>,
    pub tgmap1_w: Array2<f32>,
    pub tgmap1_n: Array2<f32>,
    pub tgout1: Array2<f32>,
}

/// Pure-ndarray implementation of batched thermal delay.
/// Callable from pipeline.rs (fused path) or from the PyO3 wrapper (modular path).
#[allow(clippy::too_many_arguments)]
pub(crate) fn ts_wave_delay_batch_pure(
    lup: ArrayView2<f32>,
    lup_e: ArrayView2<f32>,
    lup_s: ArrayView2<f32>,
    lup_w: ArrayView2<f32>,
    lup_n: ArrayView2<f32>,
    tg_temp: ArrayView2<f32>,
    firstdaytime: i32,
    timeadd: f32,
    timestepdec: f32,
    tgmap1: ArrayView2<f32>,
    tgmap1_e: ArrayView2<f32>,
    tgmap1_s: ArrayView2<f32>,
    tgmap1_w: ArrayView2<f32>,
    tgmap1_n: ArrayView2<f32>,
    tgout1: ArrayView2<f32>,
) -> TsWaveDelayBatchPureResult {
    let mut tgmap1_arr = tgmap1.to_owned();
    let mut tgmap1_e_arr = tgmap1_e.to_owned();
    let mut tgmap1_s_arr = tgmap1_s.to_owned();
    let mut tgmap1_w_arr = tgmap1_w.to_owned();
    let mut tgmap1_n_arr = tgmap1_n.to_owned();
    let mut tgout1_arr = tgout1.to_owned();

    // First morning: reset previous temperatures
    if firstdaytime == 1 {
        tgmap1_arr.assign(&lup);
        tgmap1_e_arr.assign(&lup_e);
        tgmap1_s_arr.assign(&lup_s);
        tgmap1_w_arr.assign(&lup_w);
        tgmap1_n_arr.assign(&lup_n);
        tgout1_arr.assign(&tg_temp);
    }

    let threshold = 59.0 / 1440.0;
    let decay_constant = -33.27f32;

    if timeadd >= threshold {
        let weight1 = (decay_constant * timeadd).exp();
        let new_timeadd = if timestepdec > threshold {
            timestepdec
        } else {
            0.0
        };

        let m = lerp_par(lup, tgmap1_arr.view(), weight1);
        let me = lerp_par(lup_e, tgmap1_e_arr.view(), weight1);
        let ms = lerp_par(lup_s, tgmap1_s_arr.view(), weight1);
        let mw = lerp_par(lup_w, tgmap1_w_arr.view(), weight1);
        let mn = lerp_par(lup_n, tgmap1_n_arr.view(), weight1);
        let mt = lerp_par(tg_temp, tgout1_arr.view(), weight1);

        TsWaveDelayBatchPureResult {
            lup: m.clone(),
            lup_e: me.clone(),
            lup_s: ms.clone(),
            lup_w: mw.clone(),
            lup_n: mn.clone(),
            tg_out: mt.clone(),
            timeadd: new_timeadd,
            tgmap1: m,
            tgmap1_e: me,
            tgmap1_s: ms,
            tgmap1_w: mw,
            tgmap1_n: mn,
            tgout1: mt,
        }
    } else {
        let new_timeadd = timeadd + timestepdec;
        let weight1 = (decay_constant * new_timeadd).exp();

        TsWaveDelayBatchPureResult {
            lup: lerp_par(lup, tgmap1_arr.view(), weight1),
            lup_e: lerp_par(lup_e, tgmap1_e_arr.view(), weight1),
            lup_s: lerp_par(lup_s, tgmap1_s_arr.view(), weight1),
            lup_w: lerp_par(lup_w, tgmap1_w_arr.view(), weight1),
            lup_n: lerp_par(lup_n, tgmap1_n_arr.view(), weight1),
            tg_out: lerp_par(tg_temp, tgout1_arr.view(), weight1),
            timeadd: new_timeadd,
            tgmap1: tgmap1_arr,
            tgmap1_e: tgmap1_e_arr,
            tgmap1_s: tgmap1_s_arr,
            tgmap1_w: tgmap1_w_arr,
            tgmap1_n: tgmap1_n_arr,
            tgout1: tgout1_arr,
        }
    }
}

/// Parallel weighted average: out[i] = curr[i] * (1-w) + prev[i] * w
fn lerp_par(curr: ArrayView2<f32>, prev: ArrayView2<f32>, w: f32) -> Array2<f32> {
    let w1 = 1.0 - w;
    let mut out = Array2::zeros(curr.dim());
    Zip::from(&mut out)
        .and(&curr)
        .and(&prev)
        .par_for_each(|o, &c, &p| {
            *o = c * w1 + p * w;
        });
    out
}
