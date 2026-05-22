<!-- Copyright 2026 Regit.io — Nicolas Koenig -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# regit-identifiers

Securities identifier validation. Zero-dependency, pure Rust, `no_std`.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![no_std](https://img.shields.io/badge/no__std-yes-success.svg)](https://docs.rust-embedded.org/book/intro/no-std.html)

## What it does

`regit-identifiers` parses, validates, and converts the identifiers that name
a security or a market: **ISIN**, **CUSIP** / **CINS**, **SEDOL**, **LEI**,
**BIC**, **MIC**, **FIGI**, **CFI**, and the national numbers **WKN** and
**VALOR**.

For every identifier that carries one, the check digit is recomputed from its
governing standard and verified — never trusted. Structural rules (length,
character set, segment layout, country codes) are enforced exactly as the
standard specifies. A unified [`SecurityId`](src/detect.rs) auto-detects which
kind of identifier a raw string is, and a conversion layer moves between the
ones that are formally related.

Every rule is traced to a citable standard in [SPEC.md](SPEC.md). A regulator,
an auditor, or a new engineer can open any source file and check it against
ISO 6166, ISO 17442, ISO 9362, ISO 10383, ISO 10962, or the CUSIP/SEDOL/FIGI
specifications.

## Why this crate exists

An identifier is the primary key of a security. A trade is routed, a position
is settled, a holding is reported to a regulator, and a corporate action is
applied — all against an identifier. If a malformed or mistyped identifier is
accepted as valid, every one of those operations is wrong, and the error is
silent until it is expensive.

Most identifiers defend against exactly this with a **check digit** — a
redundancy that catches the overwhelming majority of single-character typos
and digit transpositions. But the check is only as good as its
implementation, and the algorithms are deceptively easy to get subtly wrong:
ISIN expands letters to two digits *before* applying Luhn weights; FIGI doubles
from the *second*-rightmost character, not the rightmost; SEDOL uses a fixed
weight vector and, unlike the others, never splits two-digit products; LEI is a
whole-string modulus that overflows a 64-bit integer. A validator that gets any
of these wrong reports `valid` on bad data — the worst possible failure.

`regit-identifiers` implements each algorithm from its governing standard,
verifies it against hand-computed worked examples for real instruments, and
ships the verification alongside the code. It is `no_std` and allocation-free,
so the same audited logic runs in a backend service, a WASM bundle, or on an
embedded device with no change.

This sits within [Regit OS](https://www.regit.io): `regit-identifiers` is the
reference-data layer — the component that decides whether an identifier is real
before anything downstream acts on it.

## Quick start

```toml
[dependencies]
regit-identifiers = "1.0"
```

```rust
use regit_identifiers::{Isin, SecurityId};

// Parse and validate — the check digit is recomputed and verified.
let isin = Isin::parse("US0378331005").unwrap();
assert_eq!(isin.country_code(), "US");
assert_eq!(isin.nsin(), "037833100");
assert_eq!(isin.check_digit(), '5');

// A wrong check digit is rejected, not silently accepted.
assert!(Isin::parse("US0378331004").is_err());

// Auto-detect which kind of identifier a raw string is.
match SecurityId::detect("BBG000BLNNH6") {
    Some(SecurityId::Figi(figi)) => assert_eq!(figi.as_str(), "BBG000BLNNH6"),
    _ => panic!("expected a FIGI"),
}
```

See [`examples/quickstart.rs`](examples/quickstart.rs) for a complete tour
covering every identifier, the conversion layer, and the MIC registry.

## Identifiers covered

| Identifier | Standard | Length | Check digit | Module |
|---|---|---|---|---|
| ISIN | ISO 6166 | 12 | Luhn mod-10 over expanded string | [`isin`](src/isin.rs) |
| CUSIP / CINS | ANSI X9.6 | 9 | Modulus 10 double-add-double | [`cusip`](src/cusip.rs) |
| SEDOL | LSE | 7 | Weighted sum mod-10 | [`sedol`](src/sedol.rs) |
| LEI | ISO 17442 | 20 | ISO 7064 MOD 97-10 | [`lei`](src/lei.rs) |
| FIGI | ANSI X9.145 | 12 | Modulus 10 double-add-double | [`figi`](src/figi.rs) |
| BIC / SWIFT | ISO 9362 | 8 or 11 | — structural | [`bic`](src/bic.rs) |
| MIC | ISO 10383 | 4 | — structural + registry | [`mic`](src/mic.rs) |
| CFI | ISO 10962 | 6 | — structural | [`cfi`](src/cfi.rs) |
| WKN | WM Datenservice | 6 | — structural | [`wkn`](src/wkn.rs) |
| VALOR | SIX | 1–9 | — structural | [`valor`](src/valor.rs) |

## The check-digit core

The thesis of this crate is that a securities identifier is a **check-digit
scheme over an alphanumeric alphabet**. The [`checkdigit`](src/checkdigit.rs)
module exposes that core directly:

```rust
use regit_identifiers::checkdigit;

assert_eq!(checkdigit::isin_check_digit("US037833100").unwrap(), '5');
assert_eq!(checkdigit::cusip_check_digit("03783310").unwrap(),   '0');
assert_eq!(checkdigit::sedol_check_digit("026349").unwrap(),     '4');
assert_eq!(checkdigit::figi_check_digit("BBG000BLNNH").unwrap(), '6');
```

Each function is the algorithm from its standard, documented with the
derivation and a worked example.

## Conversions

The [`convert`](src/convert.rs) module moves between identifiers that are
formally related — when, and only when, the relationship is defined:

- a US/Canada **ISIN ↔ CUSIP** (the NSIN body *is* the CUSIP);
- a UK/Ireland **ISIN ↔ SEDOL** (the SEDOL is left-padded into the NSIN);
- a Swiss **ISIN ↔ VALOR**;
- building an **ISIN** from any country code plus a national number.

Conversions that have no defined meaning — a German ISIN to a CUSIP, say —
return a typed error rather than a wrong answer.

## Unified detection

[`SecurityId`](src/detect.rs) is a single type that holds any identifier, and
`SecurityId::detect` recognises which one a raw string is — by length,
structure, and, where one exists, a passing check digit. The trial order is
**LEI → ISIN → FIGI → CUSIP → SEDOL → BIC → MIC**, checksum-strength first;
the ambiguous structural-only kinds (CFI, WKN, VALOR) are not auto-detected
(parse them explicitly). It is the entry point when the kind of an identifier
is not known ahead of time.

## Architecture

```
src/
  lib.rs          # Module declarations + re-exports
  errors.rs       # Typed errors — ValidationError, ConversionError
  charset.rs      # ASCII alphabet primitives (internal)
  checkdigit.rs   # Check-digit algorithms — ISIN, CUSIP, SEDOL, LEI, FIGI

  isin.rs         # ISIN   — ISO 6166
  cusip.rs        # CUSIP and CINS
  sedol.rs        # SEDOL
  lei.rs          # LEI    — ISO 17442
  figi.rs         # FIGI   — ANSI X9.145
  bic.rs          # BIC    — ISO 9362
  mic.rs          # MIC    — ISO 10383
  cfi.rs          # CFI    — ISO 10962
  wkn.rs          # WKN    — German national number
  valor.rs        # VALOR  — Swiss national number

  country.rs      # ISO 3166-1 alpha-2 + ISIN prefix table
  mic_registry.rs # Embedded ISO 10383 registry snapshot (feature `mic-registry`)
  convert.rs      # Cross-identifier conversions
  detect.rs       # Unified SecurityId + auto-detection
```

One file, one identifier. Each type is `Copy`, allocation-free, and validated
on construction.

## Testing

```bash
cargo test                      # unit + integration + doc-tests
cargo run --example quickstart  # end-to-end tour
cargo bench                     # criterion benchmarks
```

Tests are anchored on **real, well-known instruments** with hand-computed
check digits — Apple's ISIN and CUSIP, BAE Systems' SEDOL, IBM's FIGI, a
Bloomberg LEI — together with the documented invalid cases for each standard,
`proptest` round-trip invariants, and the conversion identities.

## Code quality

- `#![no_std]`, allocation-free — runs in services, WASM, and on embedded
  targets with no `std`
- `#![forbid(unsafe_code)]` crate-wide
- `clippy::pedantic` with zero warnings
- No `unwrap()`, `expect()`, or `panic!()` in library code — every failure
  path is a typed `Result`
- Every public item documented with its governing standard and a runnable
  example
- Deterministic: the same input always produces the same verdict

## Dependencies

**Runtime: zero.** Not `std`, not `alloc`, no FFI. Every check-digit algorithm
is hand-rolled from its governing standard. Licence and supply-chain policy is
enforced via `cargo-deny` (`deny.toml`).

The default `mic-registry` feature embeds a dated snapshot of the ISO 10383
registry as static, `no_std`-clean data. Disable it for a structural-only MIC
build: `cargo build --no-default-features`.

## Standards

All algorithms implemented from their governing standard — no ports from other
implementations.

| Standard | Identifier |
|---|---|
| ISO 6166 | ISIN |
| ANSI X9.6 — CUSIP Global Services | CUSIP, CINS |
| London Stock Exchange SEDOL Masterfile | SEDOL |
| ISO 17442 / ISO 7064 MOD 97-10 | LEI |
| ISO 9362 | BIC |
| ISO 10383 | MIC |
| ANSI X9.145 / OMG (OpenFIGI) | FIGI |
| ISO 10962 | CFI |
| ISO 3166-1 alpha-2 | country codes |

## Documentation

- [SPEC.md](SPEC.md) — every structural rule and check-digit algorithm, traced
  to its standard, with worked examples
- [CHANGELOG.md](CHANGELOG.md) — release history
- [SECURITY.md](SECURITY.md) — vulnerability disclosure policy

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

```
Copyright 2026 Regit.io — Nicolas Koenig
```

---

Part of [Regit OS](https://www.regit.io) — the operating system for investment products. From Luxembourg.
