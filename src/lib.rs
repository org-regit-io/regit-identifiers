// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Securities identifier validation in pure Rust — `#![no_std]`, no `alloc`,
//! zero dependencies.
//!
//! `regit-identifiers` parses, validates, and converts the identifiers that
//! name a security or a market: ISIN, CUSIP / CINS, SEDOL, LEI, BIC, MIC,
//! FIGI, CFI, and the national numbers WKN and VALOR.
//!
//! An identifier is the primary key of a security: a trade is routed, a
//! position is settled, and a holding is reported to a regulator against it.
//! If a malformed or mistyped identifier is accepted as valid, every one of
//! those operations is wrong. This crate's first duty is therefore to never
//! certify a bad identifier as good. For every identifier that carries a
//! check digit, the digit is recomputed from its governing standard and
//! verified — never trusted. Structural rules are enforced exactly as the
//! standard specifies.
//!
//! The crate is `#![no_std]` and allocation-free: every identifier is a
//! fixed-size byte array, every algorithm is hand-rolled integer arithmetic,
//! and there is no runtime dependency. The same audited logic runs in a
//! backend service, a WASM bundle, or on an embedded device.
//!
//! # Quick start
//!
//! ```
//! use regit_identifiers::checkdigit;
//!
//! // Recompute the check digit of Apple's ISIN body — never trust the
//! // digit you were given.
//! assert_eq!(checkdigit::isin_check_digit("US037833100").unwrap(), '5');
//!
//! // A securities identifier is, at its core, a check-digit scheme over an
//! // alphanumeric alphabet — that core is exposed directly.
//! assert_eq!(checkdigit::cusip_check_digit("03783310").unwrap(), '0');
//! assert_eq!(checkdigit::sedol_check_digit("026349").unwrap(), '4');
//! assert_eq!(checkdigit::figi_check_digit("BBG000BLNNH").unwrap(), '6');
//! ```
//!
//! # Architecture
//!
//! ```text
//! errors       typed errors — ValidationError, ConversionError
//! charset      ASCII alphabet primitives (internal)
//! checkdigit   check-digit algorithms — ISIN, CUSIP, SEDOL, LEI, FIGI
//! country      ISO 3166-1 alpha-2 codes and ISIN prefix rules
//!
//! isin         ISIN  — ISO 6166         cusip   CUSIP / CINS — ANSI X9.6
//! sedol        SEDOL — LSE Masterfile   lei     LEI   — ISO 17442
//! figi         FIGI  — ANSI X9.145      bic     BIC   — ISO 9362
//! mic          MIC   — ISO 10383        cfi     CFI   — ISO 10962
//! wkn          WKN   — national (DE)    valor   VALOR — national (CH)
//!
//! convert      cross-identifier conversions (ISIN ⇄ CUSIP / SEDOL / VALOR)
//! detect       SecurityId — unified type + auto-detection
//! mic_registry embedded ISO 10383 registry  (feature `mic-registry`)
//! ```
//!
//! Every rule is traced to a citable standard — ISO 6166, ISO 17442,
//! ISO 9362, ISO 10383, ISO 10962, ISO 7064, and the CUSIP, SEDOL, and FIGI
//! specifications — in the doc comment of the module that implements it.
//!
//! Part of [Regit OS](https://www.regit.io) — the operating system for
//! investment products. From Luxembourg.

#![no_std]
#![forbid(unsafe_code)]

mod charset;

#[cfg(test)]
mod test_support;

pub mod bic;
pub mod cfi;
pub mod checkdigit;
pub mod convert;
pub mod country;
pub mod cusip;
pub mod detect;
pub mod errors;
pub mod figi;
pub mod isin;
pub mod lei;
pub mod mic;
pub mod sedol;
pub mod valor;
pub mod wkn;

/// Embedded snapshot of the ISO 10383 Market Identifier Code registry.
/// Enabled by the default `mic-registry` feature.
#[cfg(feature = "mic-registry")]
pub mod mic_registry;

pub use bic::Bic;
pub use cfi::Cfi;
pub use cusip::Cusip;
pub use detect::{IdentifierKind, SecurityId};
pub use errors::{ConversionError, ValidationError};
pub use figi::Figi;
pub use isin::Isin;
pub use lei::Lei;
pub use mic::Mic;
pub use sedol::Sedol;
pub use valor::Valor;
pub use wkn::Wkn;

/// `MicEntry` and `MicStatus` are re-exported at the crate root for
/// ergonomic access — both originate in [`mic_registry`] and travel with the
/// `mic-registry` feature.
#[cfg(feature = "mic-registry")]
pub use mic::{MicEntry, MicStatus};
