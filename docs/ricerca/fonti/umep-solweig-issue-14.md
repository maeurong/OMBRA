# Making the compute path reachable from Rust

https://github.com/UMEP-dev/solweig/issues/14 (aperta 2026-09-03T09:50:47Z)

## Corpo

Hi. I maintain [CLIMESH](https://github.com/maeurong/CLIMESH), a GPL-3 outdoor thermal comfort tool for university teaching. It ships as a single Rust binary with no native dependencies, and it reuses this repository's kernel: shadowing.rs, utci.rs, and as of this week skyview.rs. I vendor them rather than depend on the crate, at a pinned commit, with attribution and a script that does the extraction. A CI job re-runs that script against the pinned commit and diffs the result byte for byte, so the copy can't drift without someone noticing.

I'd rather use your code than write my own. The parity tests are most of why it can be trusted, and rewriting the thousand-odd lines that tie together shadows, SVF, radiation and ground thermal inertia would lose them, along with the constants that aren't in the papers.

Four things would help. The first is the one I actually need. The other three only matter if you ever want Rust consumers to depend on the crate instead of copying out of it, and the last of them is expensive.

### 1. Give the timestep physics a name

compute_timestep in rust/src/pipeline.rs is a #[pyfunction], but the physics is already separate from the binding. Lines 502-708 bind the numpy arrays, the closure at 709-1954 is pure Rust and returns TimestepResultRaw, and from 1955 the result goes back to Python. I searched the closure: across 1246 lines, py appears once, on the allow_threads_unchecked line that opens it.

Could that closure become a plain function, say compute_timestep_rust, taking the views and the scalar structs and returning TimestepResultRaw, with the current #[pyfunction] calling it? Behaviour and Python API unchanged. calculate_shadows_rust already has that shape, and TimestepResultRaw is already the internal type, so there's nothing to design.

It's the one piece I can't vendor. My script works by naming things, and a closure has no name. Everything else in the crate does have one. The *_pure functions in gvf, sky, ground, ground_surface, vegetation and tmrt, and pipeline_radiation.rs with no Python in it at all, made this much easier than I expected.

### 2. crate-type = ["cdylib", "rlib"]

One line in rust/Cargo.toml. It doesn't affect the Python build.

### 3. Something public to call

Every module in lib.rs is private and the compute functions are pub(crate), which is right for a crate that's only a Python extension. A pub mod on the physics modules, or one pub mod api re-exporting whatever you're willing to stand behind, would put the line where you want it and keep the rest private.

### 4. pyo3 and numpy behind a default-on feature

This is the costly one, and I'd rather say so than let you find out. 41 #[pyfunction]s, 26 #[pyclass]es and 16 #[pymethods] blocks across 17 of the 23 modules would need #[cfg(feature = "python")], plus the binding code in pipeline.rs. With extension-module on, the cdylib doesn't link libpython; with it off, it does, which is what keeps the crate out of a static binary today.

I'm not asking for this. I mention it because 2 and 3 are cheap but useless without it, so if 4 is out then skipping those is reasonable too, and I'll keep vendoring, which works.

I can send PRs for 1 and 2, done mechanically with your tests green. 3 I'd rather leave to you, since it's a decision about what you want to commit to rather than a refactor.

If 1 doesn't happen I'll extend my extraction script to pipeline.rs instead. It's regular enough to script, just more fragile, and that's my problem rather than yours.

## Commento di songololo (2026-11-10T17:37:03Z)

Thanks for the careful write-up, and apologies for the slow reply. Reusing the code with the parity tests attached is what I would want too.

Taking your four points in order:

1. **Named function for the timestep physics.** Yes, and I will do this one here. It is a move of about 1,250 lines in the file that changes most often (`pipeline.rs` changed again in 0.1.0b97 and b98), and it has to leave the golden and parity outputs byte-identical, which is quickest to check on this side with the GPU tests. It will be in the next release.
2. **`crate-type = ["cdylib", "rlib"]`.** Yes, in the same release.
3. **Public API.** I would rather add a small `pub mod api` that re-exports what you actually call than make the physics modules `pub` wholesale. The internal signatures still move between releases, and a narrow surface is one I can keep stable. Could you list the functions CLIMESH uses today from `shadowing.rs`, `utci.rs` and `skyview.rs`, plus what you need from the timestep function in point 1? I will start from that list.
4. **pyo3 and numpy behind a feature flag.** Agreed that this is the expensive one. I am not planning it, but I am open to a PR if you want to take it on once points 1 to 3 are in.

Two things that may affect what you have vendored:

- 0.1.0b97 fixed an anisotropic-sky fault in `skyview.rs` for surfaces without vegetation (issue #15). Before that, the vegetation shadow matrices it returns without a canopy layer are wrong for the anisotropic sky. The SVF values themselves are unaffected.
- 0.1.0b98 fixed three cases where the GPU contexts reused inputs uploaded for a previous surface of the same grid size. If you use the GPU shadow path from `shadowing.rs` across more than one surface in a process, please update.

The reference implementation this package tracks is now [UMEP-dev/umep-solweig](https://github.com/UMEP-dev/umep-solweig). Physical-law tests are being added there, and this repository keeps the tests that check the Rust against it.


## Commento di maeurong (2026-11-10T18:08:11Z)

Thanks, this is more than I hoped for, and no apology needed. You doing point 1 on your side is the right call: you have the GPU tests and I don't.

Here is what my project calls today:

- `shadowing::calculate_shadows_rust`, and I read the fields of `ShadowingResultRust`
- `skyview::calculate_svf_inner`, and I read the fields of `SvfIntermediate`
- `utci::utci_single`

That's the whole list. My copy also exposes `skyview::crop_svf_intermediate`, but nothing calls it, so leave it out.

For the timestep function I need three things: the function itself, `TimestepResultRaw` with fields I can read, and the structs it takes as arguments, in a form I can build from outside the crate. I can't be more precise until I see the signature, so pick whatever shape suits you and I'll adapt.

On the two fixes: I run on CPU only, so the b98 one doesn't reach me. The b97 one would have. So far I only read the SVF values from `skyview.rs`, but the anisotropic sky is my next step and it needs those shadow matrices, so I'll move my pin to b98 first. Thanks for telling me before I found out the hard way.

I'll leave point 4 alone. Vendoring works, and points 1 to 3 take away the fragile part of it.


## Commento di songololo (2026-11-11T12:24:27Z)

Points 1 to 3 are done: commit `92c7b8f` on `main`, released as 0.1.0b99 (tag `v0.1.0b99`).

**What is public.** One module, `rustalgos::api`, re-exports everything. The modules themselves stay private.

```rust
use rustalgos::api::{
    // shadows
    calculate_shadows_rust, ShadowingResultRust,
    // sky view factor and shadow matrices
    calculate_svf_inner, SvfIntermediate,
    // UTCI
    utci_single,
    // one timestep
    compute_timestep_rust, TimestepInputs, TimestepResultRaw,
    SurfaceViews, SvfViews, PropertyViews, ThermalStateViews, GroundSchemeViews,
    WeatherScalars, HumanScalars, ConfigScalars,
    OUT_ALL, OUT_SHADOW, OUT_KDOWN, OUT_KUP, OUT_LDOWN, OUT_LUP,
    // ground view factor geometry, once per surface
    precompute_gvf_geometry_cpu, GvfGeometryCache,
};
```

The three functions you call today keep their signatures; they are only `pub` now. `crop_svf_intermediate` is left out as you asked.

**The timestep function.**

```rust
pub fn compute_timestep_rust(inputs: &TimestepInputs) -> Result<TimestepResultRaw, String>
```

- `TimestepInputs` and the structs inside it (`rust/src/pipeline_inputs.rs`) hold `ndarray::ArrayView2<f32>` fields, all `pub`, so you build them with struct literals from your own arrays. No numpy types.
- `WeatherScalars`, `HumanScalars` and `ConfigScalars` are the same structs the Python side uses. Their fields are `pub` and each has a `new(...)`.
- `TimestepResultRaw` has `pub` fields: `tmrt` always, `shadow` / `kdown` / `kup` / `ldown` / `lup` as `Option`s selected by `inputs.output_mask`, and the thermal state (`timeadd`, `tgmap1*`, `tgout1`) to pass into the next step through `ThermalStateViews`.
- `gvf_cache` is optional. Build it once per surface with `precompute_gvf_geometry_cpu` and pass `Some(&cache)` on every step; `None` repeats the building ray-trace each step. `GvfGeometryCache` is opaque.
- For the anisotropic sky, pass `shmat` / `vegshmat` / `vbshmat` straight from `SvfIntermediate` (`bldg_sh_matrix`, `veg_sh_matrix`, `veg_blocks_bldg_sh_matrix`). They are bitpacked, `ceil(n_patches / 8)` bytes deep.
- It returns `Err` for missing wall rasters with `has_walls` set, and for a shadow matrix whose shape does not match the DSM or the patch option. Every other raster is assumed to have the DSM's shape; that is not checked.

**How it was checked.** The body is a verbatim move: the full test suite passes with no golden fixture changed. I also type-checked a separate crate that depends on this one with `default-features = false` (CPU only), builds every input struct, calls all four entry points and reads the result fields.

**Two limits.**

- `calculate_svf_inner` still returns `PyResult`, and the scalar structs are `#[pyclass]`, so pyo3 stays in the dependency tree. That is point 4.
- The crate type now includes `rlib`, but `pyo3` is built with `extension-module`, so a binary that links this crate directly will not find the Python symbols. `cargo check` and vendoring are unaffected.

I will treat changes to anything in `api` as breaking and note them in the release notes.

**Pin 0.1.0b100, not b99.** It came out the same day and has no change to the `api` signatures, but two things in it affect `compute_timestep_rust`:

- `HumanScalars.height` sets the ground view factor's search range. Pass the height of the body's centre of gravity for the posture (1.1 m standing, 0.75 m sitting), as the reference does, not the stature. The Python side passed 1.75 m up to b99.
- With a `gvf_cache`, ground-reflected shortwave and ground-emitted longwave were too low within the search range of the raster edge (22 m standing). The steps of the search that left the raster were dropped; the reference keeps the last value seen. Fixed in b100.


## Commento di maeurong (2026-11-11T16:18:59Z)

Thank you, this is exactly what I needed. I've moved my pin to b101, and I'll pass the centre-of-gravity height as you advised.


