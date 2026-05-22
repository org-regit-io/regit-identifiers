// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Criterion benchmarks for regit-identifiers.
//!
//! Every operation in this crate is fixed-size, allocation-free integer
//! arithmetic over a short byte array, so all of it is comfortably sub-microsecond.
//!
//! Performance targets (indicative, native release on commodity hardware):
//!
//! | Operation                          | Target   |
//! |------------------------------------|----------|
//! | Luhn / SEDOL / CUSIP check digit   | < 30 ns  |
//! | ISIN / FIGI check digit (expanded) | < 50 ns  |
//! | LEI check digits (MOD 97-10)       | < 60 ns  |
//! | Identifier `parse` (any type)      | < 120 ns |
//! | `SecurityId::detect`               | < 500 ns |
//! | MIC registry `lookup` (2.8k rows)  | < 120 ns |

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use regit_identifiers::checkdigit;
use regit_identifiers::detect::SecurityId;
use regit_identifiers::{Bic, Cfi, Cusip, Figi, Isin, Lei, Mic, Sedol, Valor, Wkn};

// ─── Check-digit computation ─────────────────────────────────────────────────

fn bench_checkdigit(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkdigit");

    group.bench_function("luhn", |b| {
        b.iter(|| checkdigit::luhn_checksum(black_box("7992739871")));
    });
    group.bench_function("isin", |b| {
        b.iter(|| checkdigit::isin_check_digit(black_box("US037833100")));
    });
    group.bench_function("cusip", |b| {
        b.iter(|| checkdigit::cusip_check_digit(black_box("03783310")));
    });
    group.bench_function("sedol", |b| {
        b.iter(|| checkdigit::sedol_check_digit(black_box("026349")));
    });
    group.bench_function("lei", |b| {
        b.iter(|| checkdigit::lei_check_digits(black_box("5493001KJTIIGC8Y1R")));
    });
    group.bench_function("figi", |b| {
        b.iter(|| checkdigit::figi_check_digit(black_box("BBG000BLNNH")));
    });

    group.finish();
}

// ─── Parsing ─────────────────────────────────────────────────────────────────

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    group.bench_function("isin", |b| {
        b.iter(|| Isin::parse(black_box("US0378331005")));
    });
    group.bench_function("cusip", |b| {
        b.iter(|| Cusip::parse(black_box("037833100")));
    });
    group.bench_function("sedol", |b| {
        b.iter(|| Sedol::parse(black_box("0263494")));
    });
    group.bench_function("lei", |b| {
        b.iter(|| Lei::parse(black_box("5493001KJTIIGC8Y1R12")));
    });
    group.bench_function("figi", |b| {
        b.iter(|| Figi::parse(black_box("BBG000BLNNH6")));
    });
    group.bench_function("bic", |b| {
        b.iter(|| Bic::parse(black_box("DEUTDEFF500")));
    });
    group.bench_function("mic", |b| {
        b.iter(|| Mic::parse(black_box("XNAS")));
    });
    group.bench_function("cfi", |b| {
        b.iter(|| Cfi::parse(black_box("ESVUFR")));
    });
    group.bench_function("wkn", |b| {
        b.iter(|| Wkn::parse(black_box("A1EWWW")));
    });
    group.bench_function("valor", |b| {
        b.iter(|| Valor::parse(black_box("1213853")));
    });

    group.finish();
}

// ─── Auto-detection ──────────────────────────────────────────────────────────

fn bench_detect(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect");

    // The LEI branch is tried first, so an LEI is the cheapest detection.
    group.bench_function("lei", |b| {
        b.iter(|| SecurityId::detect(black_box("5493001KJTIIGC8Y1R12")));
    });
    // A BIC falls through every checksum-bearing branch before matching.
    group.bench_function("bic", |b| {
        b.iter(|| SecurityId::detect(black_box("DEUTDEFF")));
    });
    // Garbage exercises every branch and matches none — the worst case.
    group.bench_function("miss", |b| {
        b.iter(|| SecurityId::detect(black_box("not-an-identifier")));
    });

    group.finish();
}

// ─── MIC registry lookup ─────────────────────────────────────────────────────

#[cfg(feature = "mic-registry")]
fn bench_registry(c: &mut Criterion) {
    use regit_identifiers::mic_registry::lookup;

    let mut group = c.benchmark_group("registry");

    // A binary search over the embedded ISO 10383 snapshot.
    group.bench_function("lookup_hit", |b| {
        b.iter(|| lookup(black_box("XNAS")));
    });
    group.bench_function("lookup_miss", |b| {
        b.iter(|| lookup(black_box("ZZZZ")));
    });
    group.bench_function("parse_registered", |b| {
        b.iter(|| Mic::parse_registered(black_box("XLON")));
    });

    group.finish();
}

// ─── Harness ─────────────────────────────────────────────────────────────────

#[cfg(feature = "mic-registry")]
criterion_group!(
    benches,
    bench_checkdigit,
    bench_parse,
    bench_detect,
    bench_registry,
);

#[cfg(not(feature = "mic-registry"))]
criterion_group!(benches, bench_checkdigit, bench_parse, bench_detect);

criterion_main!(benches);
