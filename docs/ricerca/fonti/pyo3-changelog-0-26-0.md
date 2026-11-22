# Estratto: CHANGELOG di PyO3, voce 0.26.0

Righe 528-540 di `CHANGELOG.md` sul ramo `main` di PyO3/pyo3, scaricato il 2026-11-17
(https://raw.githubusercontent.com/PyO3/pyo3/main/CHANGELOG.md). Testo riportato senza modifiche.

```
## [0.26.0] - 2025-08-29

### Packaging

- Bump hashbrown dependency to 0.15. [#5152](https://github.com/PyO3/pyo3/pull/5152)
- Update MSRV to 1.74. [#5171](https://github.com/PyO3/pyo3/pull/5171)
- Set the same maximum supported version for alternative interpreters as for CPython. [#5192](https://github.com/PyO3/pyo3/pull/5192)
- Add optional `bytes` dependency to add conversions for `bytes::Bytes`. [#5252](https://github.com/PyO3/pyo3/pull/5252)
- Publish new crate `pyo3-introspection` to pair with the `experimental-inspect` feature. [#5300](https://github.com/PyO3/pyo3/pull/5300)
- The `PYO3_BUILD_EXTENSION_MODULE` now causes the same effect as the `extension-module` feature. Eventually we expect maturin and setuptools-rust to set this environment variable automatically. Users with their own build systems will need to do the same. [#5343](https://github.com/PyO3/pyo3/pull/5343)

### Added
```
