<!-- Copyright 2026 Regit.io — Nicolas Koenig -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.x     | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in `regit-identifiers`, please report
it responsibly:

1. **Do not** open a public GitHub issue
2. Email **nicolas.koenig@regit.io** with a description of the vulnerability
3. Include steps to reproduce if possible
4. We will acknowledge receipt within 48 hours and provide a timeline for a fix

## Scope

This crate performs validation and parsing of securities identifiers only. It
is `no_std`, allocation-free, and performs no network I/O, no file I/O, no
authentication, and no external communication. It has zero runtime
dependencies.

The primary security concern is **correctness of the verdict**. A
**false positive** — reporting a malformed or mistyped identifier as valid —
is treated as a defect of the highest severity, because a downstream system
may route a trade, settle a position, or file a regulatory report against an
identifier the crate wrongly certified. An incorrect check-digit computation,
an incorrect character-set rule, or an incorrect structural rule all fall in
this class.

If you find any input where a validity verdict, a check-digit computation, or
a conversion result is incorrect with respect to the governing standard, please
report it using the process above. Each rule is traced to its standard in
[SPEC.md](SPEC.md).

## Dependencies

The crate has no runtime dependencies. Licence and supply-chain concerns for
development dependencies are policed via `cargo-deny` (`deny.toml` in the
repository root), checked in CI on every push. Dependency changes that
introduce a non-allowed licence or an active advisory are rejected at the gate.
