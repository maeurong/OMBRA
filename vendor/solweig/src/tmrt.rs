use ndarray::{Array2, ArrayView2};
use rayon::prelude::*;

/// Physical constants
const SBC: f32 = 5.67051e-8; // Stefan-Boltzmann constant (W/m²/K⁴)
const KELVIN_OFFSET: f32 = 273.15; // Kelvin to Celsius conversion

/// View factors for standing posture
const F_UP_STANDING: f32 = 0.06;
const F_SIDE_STANDING: f32 = 0.22;
const F_CYL_STANDING: f32 = 0.28;

/// View factors for sitting posture
const F_UP_SITTING: f32 = 0.166666;
const F_SIDE_SITTING: f32 = 0.166666;
const F_CYL_SITTING: f32 = 0.20;

/// Pure-ndarray implementation of Tmrt calculation.
/// Callable from pipeline.rs (fused path) or from the PyO3 wrapper (modular path).
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_tmrt_pure(
    kdown: ArrayView2<f32>,
    kup: ArrayView2<f32>,
    ldown: ArrayView2<f32>,
    lup: ArrayView2<f32>,
    kside_n: ArrayView2<f32>,
    kside_e: ArrayView2<f32>,
    kside_s: ArrayView2<f32>,
    kside_w: ArrayView2<f32>,
    lside_n: ArrayView2<f32>,
    lside_e: ArrayView2<f32>,
    lside_s: ArrayView2<f32>,
    lside_w: ArrayView2<f32>,
    kside_total: ArrayView2<f32>,
    lside_total: ArrayView2<f32>,
    abs_k: f32,
    abs_l: f32,
    is_standing: bool,
    use_anisotropic_sky: bool,
) -> Array2<f32> {
    let kside_dirs_sum = &kside_n + &kside_e + &kside_s + &kside_w;
    let lside_dirs_sum = &lside_n + &lside_e + &lside_s + &lside_w;

    compute_tmrt_from_dir_sums_pure(
        kdown,
        kup,
        ldown,
        lup,
        kside_dirs_sum.view(),
        lside_dirs_sum.view(),
        kside_total,
        lside_total,
        abs_k,
        abs_l,
        is_standing,
        use_anisotropic_sky,
    )
}

/// Tmrt kernel using pre-summed directional side components.
///
/// This avoids carrying four directional arrays through the pipeline when only
/// their sum is needed by the final Tmrt equation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_tmrt_from_dir_sums_pure(
    kdown: ArrayView2<f32>,
    kup: ArrayView2<f32>,
    ldown: ArrayView2<f32>,
    lup: ArrayView2<f32>,
    kside_dirs_sum: ArrayView2<f32>,
    lside_dirs_sum: ArrayView2<f32>,
    kside_total: ArrayView2<f32>,
    lside_total: ArrayView2<f32>,
    abs_k: f32,
    abs_l: f32,
    is_standing: bool,
    use_anisotropic_sky: bool,
) -> Array2<f32> {
    let shape = kdown.dim();

    // Select view factors based on posture
    let (f_up, f_side, f_cyl) = if is_standing {
        (F_UP_STANDING, F_SIDE_STANDING, F_CYL_STANDING)
    } else {
        (F_UP_SITTING, F_SIDE_SITTING, F_CYL_SITTING)
    };

    // Allocate output array
    let mut tmrt = Array2::<f32>::zeros(shape);

    // Compute Tmrt element-wise in parallel.
    // tmrt was just allocated via Array2::zeros so as_slice_mut() is Some.
    tmrt.as_slice_mut()
        .expect("tmrt: freshly-allocated Array2::zeros is contiguous")
        .par_iter_mut()
        .enumerate()
        .for_each(|(idx, out)| {
            let row = idx / shape.1;
            let col = idx % shape.1;

            // Skip NaN pixels — upstream NaN propagates as NaN output
            let kdown_val = kdown[[row, col]];
            if !kdown_val.is_finite() {
                *out = f32::NAN;
                return;
            }
            let kup_val = kup[[row, col]];
            let ldown_val = ldown[[row, col]];
            let lup_val = lup[[row, col]];
            let kside_dirs_sum_val = kside_dirs_sum[[row, col]];
            let lside_dirs_sum_val = lside_dirs_sum[[row, col]];
            let kside_total_val = kside_total[[row, col]];
            let lside_total_val = lside_total[[row, col]];

            // Compute absorbed shortwave radiation (same formula for both sky models)
            let k_absorbed = abs_k
                * (kside_total_val * f_cyl
                    + (kdown_val + kup_val) * f_up
                    + kside_dirs_sum_val * f_side);

            let l_absorbed = if use_anisotropic_sky {
                abs_l
                    * ((ldown_val + lup_val) * f_up
                        + lside_total_val * f_cyl
                        + lside_dirs_sum_val * f_side)
            } else {
                abs_l * ((ldown_val + lup_val) * f_up + lside_dirs_sum_val * f_side)
            };

            // Total absorbed radiation (Sstr)
            let sstr = k_absorbed + l_absorbed;

            // Convert to Tmrt using Stefan-Boltzmann law
            // Tmrt = (Sstr / (abs_l × SBC))^0.25 - 273.15
            let tmrt_val = (sstr / (abs_l * SBC)).sqrt().sqrt() - KELVIN_OFFSET;

            // Clip to physically reasonable range
            *out = tmrt_val.clamp(-50.0, 80.0);
        });

    tmrt
}
