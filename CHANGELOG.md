<!-- Copyright 2026 Regit.io — Nicolas Koenig -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2026-07-13

### Changed

- Bumped the `criterion` dev/bench dependency from 0.5 to 0.8. This is a
  dev/bench-only change: no runtime code, public API, or `no_std` behaviour
  is affected.

## [1.0.0] - 2026-05-23

First public release. The crate is `#![no_std]`, allocation-free, and has
zero runtime dependencies; every check-digit algorithm and structural rule
is hand-rolled from its governing standard and traced in [SPEC.md](SPEC.md).

### Added — identifier types

- `Isin` — ISO 6166. 12 characters; country prefix, NSIN, Luhn-on-expanded
  check digit. Accepts ISO 3166-1 alpha-2 codes and the ISIN substitute
  prefixes (`XS`, `EU`, `XA`, `XB`, `XC`, `XD`, `XF`, `XK`, `QS`, `QT`).
- `Cusip` — ANSI X9.6. 9 characters; modulus-10 double-add-double check
  digit; CINS support via `is_cins()` / `is_domestic()` / `cins_region()`
  (23-letter region table from the CUSIP Global Services specification).
- `Sedol` — London Stock Exchange. 7 characters; digits-and-consonants body
  (vowels rejected); fixed `[1,3,1,7,3,9]` weight vector, no digit folding;
  `is_legacy_numeric()`.
- `Lei` — ISO 17442 / ISO 7064 MOD 97-10. 20 characters; reserved `"00"` at
  positions 5–6; streaming modulus that never builds a wide integer; first
  differing check digit reported on mismatch.
- `Figi` — ANSI X9.145 / OpenFIGI. 12 characters; two-consonant provider
  prefix with the seven forbidden prefixes that would collide with ISIN
  country codes, literal `'G'` at position 3, no-vowel body, right-to-left
  modulus-10 with the rightmost character at weight 1.
- `Bic` — ISO 9362. 8 or 11 characters; variable-length storage with the
  unused tail zeroed; `has_branch()`, `is_test_bic()`, `is_passive()`,
  `location_status()` (the raw "status character", char 8).
- `Mic` — ISO 10383. 4 characters; structural validation, and behind the
  default `mic-registry` feature `lookup()`, `is_registered()`, and
  `parse_registered()`. `MicEntry` and `MicStatus` are re-exported at the
  crate root (gated by the same feature) for ergonomic access.
- `Cfi` — ISO 10962. 6 characters; the 14 ISO 10962 category letters
  (`category_name()`); group and attributes exposed but not validated
  against per-category tables (a documented scope bound).
- `Wkn` — Wertpapierkennnummer (WM Datenservice). 6 characters; digit or
  `A`–`Z` excluding `I` and `O`; `is_numeric()`.
- `Valor` — Valorennummer (SIX Financial Information). 1–9 digits;
  variable-length storage; `as_u64()` and the narrower `as_u32()` (the
  maximum value always fits in 32 bits).

### Added — check-digit core (`checkdigit`)

- `luhn_checksum`, `isin_check_digit`, `cusip_check_digit`,
  `sedol_check_digit`, `lei_check_digits`, `figi_check_digit`. Each
  algorithm is implemented exactly from its governing standard, verified
  against hand-computed worked examples for real instruments, and exposed as
  a public function for direct use.

### Added — conversions (`convert`)

- `isin_to_cusip` / `cusip_to_isin` — US/CA ISINs only.
- `isin_to_sedol` / `sedol_to_isin` — GB/IE ISINs; the SEDOL sits
  right-aligned, left-padded with `00` in the NSIN field.
- `isin_to_valor` / `valor_to_isin` — CH/LI ISINs.
- `build_isin(country, nsin)` — assemble any ISIN from a country prefix and
  a national number; check digit computed from the standard.
- Every cross-conversion validates the country prefix and re-runs the
  target's check digit; a conversion with no defined meaning returns a
  typed `ConversionError`, never a wrong answer.

### Added — unified detection (`detect`)

- `SecurityId` — a `Copy` enum holding any one identifier.
- `SecurityId::detect(s)` — auto-detects which kind of identifier a raw
  string is, by length, structure, and (where one exists) a passing check
  digit. Priority is checksum-strength-first: LEI, ISIN, FIGI, CUSIP,
  SEDOL, BIC, MIC. Ambiguous structural-only kinds (CFI, WKN, VALOR) are
  not auto-detected — parse them explicitly.
- `IdentifierKind`, `SecurityId::kind`, `SecurityId::as_str`.

### Added — reference data

- `country` — the 249 currently assigned ISO 3166-1 alpha-2 codes,
  `is_iso_country`, `country_name`, `is_isin_prefix`, and
  `isin_prefix_name` (resolves both ISO countries and the ten ISIN
  substitute prefixes — `XS`, `EU`, `XK`, …).
- `mic_registry` — the ISO 10383 Market Identifier Code registry,
  snapshot **2026-05-22**, **2,853 entries** (2,289 active), sorted for
  binary search. Behind the default `mic-registry` feature.

### Added — typed errors (`errors`)

- `ValidationError` — `Empty`, `WrongLength`, `InvalidCharacter`,
  `BadCheckDigit`, `InvalidCountryCode`, `Structure { rule }`.
- `ConversionError` — `UnsupportedCountry`, `NotConvertible { reason }`,
  `Validation(ValidationError)`.
- Both implement `core::fmt::Display` and `core::error::Error`;
  `ConversionError` implements `From<ValidationError>` and chains `source()`.

### Added — tests, example, benchmarks

- **493 tests** across the crate: 300 inline unit tests + 64 integration
  tests (`tests/integration.rs` — `golden`, `invalid`, `roundtrip`,
  `convert`, `detect`, `registry`, `properties` with `proptest`) + 129
  doc-tests. With `--no-default-features`: 283 + 57 + 125 = 465.
- Every check-digit algorithm was cross-validated against an independent
  oracle — Python's `python-stdnum` — on the worked examples, the golden
  vectors, and the corrected errata cases. All three implementations agree.
- Golden vectors are real, well-known instruments (Apple `US0378331005`,
  Microsoft `US5949181045`, BAE `GB0002634946`, Bayer `DE000BAY0017`,
  Bloomberg LEI, IBM FIGI, Deutsche Bank BIC, NASDAQ MIC).
- **`examples/quickstart.rs`** — a full tour of every identifier, the
  check-digit core, the conversion layer, and `SecurityId::detect`.
- **`benches/identifiers.rs`** — Criterion benchmarks for check-digit
  computation, parsing, detection, and registry lookup, with indicative
  sub-microsecond targets.

### Crate metadata

- `edition = "2024"`, MSRV `1.85`, pinned toolchain `1.95.0`.
- `#![no_std]`, `#![forbid(unsafe_code)]`, no `alloc`.
- `clippy::pedantic` clean across the whole workspace and all targets.
- Builds for `wasm32-unknown-unknown` and `thumbv7em-none-eabi` (a target
  with no `std` at all — proof of `no_std`-ness).
- Zero runtime dependencies; licence and supply-chain policy enforced via
  `cargo-deny` (`deny.toml`).

[1.0.1]: https://github.com/org-regit-io/regit-identifiers/releases/tag/v1.0.1
[1.0.0]: https://github.com/org-regit-io/regit-identifiers/releases/tag/v1.0.0
