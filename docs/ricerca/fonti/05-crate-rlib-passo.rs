// Smallest call of compute_timestep_rust that links: 12x12 grid, one 6 m block,
// isotropic sky, no vegetation, no walls. Writes every output raster as raw
// little-endian f32 to $PROBE_OUT so two builds can be compared byte for byte.
use ndarray::Array2;
use rustalgos::api::*;

#[test]
fn passo() {
    let (r, c) = (12, 12);
    let z = Array2::<f32>::zeros((r, c));
    let f = |v: f32| Array2::<f32>::from_elem((r, c), v);
    let mut dsm = f(100.0);
    let mut buildings = f(1.0);
    for i in 4..7 {
        for j in 4..7 {
            dsm[[i, j]] = 106.0;
            buildings[[i, j]] = 0.0;
        }
    }
    let svf = f(0.8);
    let svfalfa = f(0.4);
    let valid = Array2::<u8>::from_elem((r, c), 1);
    let (alb, emis, tgk, tstart, tmax) = (f(0.15), f(0.95), f(0.37), f(-3.41), f(15.0));

    let weather = WeatherScalars::new(
        200.0, 45.0, 45.0, 25.0, 50.0, 800.0, 600.0, 200.0, 65.0, 0.8, 180.5, 5.0, 900.0, 45.0,
        0.03, true, 180, 2,
    );
    let human = HumanScalars::new(1.1, 0.7, 0.95, true);
    let config = ConfigScalars::new(
        1.0, 6.0, 0.2, 0.9, 0.37, -3.41, 15.0, false, false, false, false, 500.0,
    );
    let inputs = TimestepInputs {
        weather: &weather,
        human: &human,
        config: &config,
        gvf_cache: None,
        surface: SurfaceViews {
            dsm: dsm.view(),
            cdsm: None,
            tdsm: None,
            bush: None,
            wall_ht: None,
            wall_asp: None,
        },
        svf: SvfViews {
            svf: svf.view(),
            svf_n: svf.view(),
            svf_e: svf.view(),
            svf_s: svf.view(),
            svf_w: svf.view(),
            svf_veg: svf.view(),
            svf_veg_n: svf.view(),
            svf_veg_e: svf.view(),
            svf_veg_s: svf.view(),
            svf_veg_w: svf.view(),
            svf_aveg: svf.view(),
            svf_aveg_n: svf.view(),
            svf_aveg_e: svf.view(),
            svf_aveg_s: svf.view(),
            svf_aveg_w: svf.view(),
            svfbuveg: svf.view(),
            svfalfa: svfalfa.view(),
        },
        properties: PropertyViews {
            alb_grid: alb.view(),
            emis_grid: emis.view(),
            tgk_grid: tgk.view(),
            tstart_grid: tstart.view(),
            tmaxlst_grid: tmax.view(),
        },
        buildings: buildings.view(),
        lc_grid: None,
        shmat: None,
        vegshmat: None,
        vbshmat: None,
        state: ThermalStateViews {
            firstdaytime: 1,
            timeadd: 0.0,
            timestep_dec: 1.0 / 24.0,
            tgmap1: z.view(),
            tgmap1_e: z.view(),
            tgmap1_s: z.view(),
            tgmap1_w: z.view(),
            tgmap1_n: z.view(),
            tgout1: z.view(),
        },
        ground_scheme: None,
        valid_mask: valid.view(),
        output_mask: OUT_ALL,
    };
    let o = compute_timestep_rust(&inputs).unwrap();
    let mut bytes = Vec::new();
    for a in [
        &o.tmrt,
        o.shadow.as_ref().unwrap(),
        o.kdown.as_ref().unwrap(),
        o.kup.as_ref().unwrap(),
        o.ldown.as_ref().unwrap(),
        o.lup.as_ref().unwrap(),
        &o.tgmap1,
        &o.tgmap1_e,
        &o.tgmap1_s,
        &o.tgmap1_w,
        &o.tgmap1_n,
        &o.tgout1,
    ] {
        for v in a.iter() {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&o.timeadd.to_le_bytes());
    assert!(o.tmrt.iter().all(|v| v.is_finite()));
    let sh = o.shadow.as_ref().unwrap();
    eprintln!("tmrt min={} max={} shadow sum={}", o.tmrt.iter().cloned().fold(f32::MAX, f32::min), o.tmrt.iter().cloned().fold(f32::MIN, f32::max), sh.sum());
    if let Ok(p) = std::env::var("PROBE_OUT") {
        std::fs::write(p, bytes).unwrap();
    }
}
