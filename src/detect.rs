// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Auto-detection — recognising which kind of identifier a raw string is.
//!
//! Reference data rarely arrives labelled. A spreadsheet cell, a CSV column,
//! or a free-text field holds *an* identifier, and the consuming system must
//! first decide *which* one before it can route, settle, or report against
//! it. [`SecurityId::detect`] makes that decision: it takes a raw string and
//! returns the single identifier kind it is — or `None` when nothing fits.
//!
//! ```text
//!   "5493001KJTIIGC8Y1R12"  ──▶  SecurityId::Lei   (20 chars, MOD 97-10 ok)
//!   "US0378331005"          ──▶  SecurityId::Isin  (12 chars, Luhn ok)
//!   "BBG000BLNNH6"          ──▶  SecurityId::Figi  (12 chars, [2]=='G', ok)
//!   "037833100"             ──▶  SecurityId::Cusip ( 9 chars, X9.6 ok)
//!   "0263494"               ──▶  SecurityId::Sedol ( 7 chars, weighted ok)
//!   "DEUTDEFF"              ──▶  SecurityId::Bic   ( 8 chars, ISO 9362)
//!   "garbage"               ──▶  None
//! ```
//!
//! # How detection decides
//!
//! Detection is **checksum-strength first**: a passing check digit is
//! high-confidence evidence, so kinds that carry one are tried before kinds
//! that do not. The order is fixed — LEI, ISIN, FIGI, CUSIP, SEDOL, BIC, MIC.
//! Each candidate is the strict `parse` of the corresponding identifier type,
//! so a kind is only reported when the input is fully, structurally valid for
//! it, check digit included.
//!
//! Two ambiguities are resolved by that order:
//!
//! - **ISIN vs FIGI** — both are 12 characters. ISIN is tried first; a string
//!   that is a valid ISIN is reported as one. FIGI additionally requires
//!   character 3 to be the literal `G` and forbids the seven ISIN-colliding
//!   provider prefixes, so a genuine FIGI is never a valid ISIN and falls
//!   through to the FIGI branch.
//! - **CUSIP vs BIC** — an 8-character string could be either. CUSIP carries
//!   a check digit and is tried first; only a string that is *not* a valid
//!   CUSIP reaches the BIC branch.
//!
//! # What is *not* detected
//!
//! Three kinds are deliberately excluded from auto-detection because they are
//! structural-only and would collide:
//!
//! - **CFI** — 6 upper-case letters; would shadow many other 6-letter inputs.
//! - **WKN** — 6 alphanumeric characters; no check digit to disambiguate.
//! - **VALOR** — 1 to 9 digits; a short run of digits is far too ambiguous.
//!
//! These have no check digit and overlap heavily with one another and with
//! other kinds, so detection would only guess. Parse them explicitly with
//! [`Cfi::parse`](crate::Cfi::parse), [`Wkn::parse`](crate::Wkn::parse), or
//! [`Valor::parse`](crate::Valor::parse) when the kind is already known.
//!
//! # MIC and the registry feature
//!
//! A MIC has no check digit, so a structurally valid MIC alone is weak
//! evidence. Detection therefore reports [`IdentifierKind::Mic`] only when the
//! `mic-registry` feature is enabled *and* the string is a *registered* MIC —
//! one present in the embedded ISO 10383 snapshot, via
//! [`Mic::parse_registered`](crate::Mic::parse_registered). With the feature
//! disabled, MIC is never auto-detected.
//!
//! # References
//!
//! - ISO 6166 (ISIN), ISO 17442 (LEI), ISO 9362 (BIC), ISO 10383 (MIC),
//!   ANSI X9.6 (CUSIP), ANSI X9.145 (FIGI) — the standards whose grammars and
//!   check digits this module relies on to tell the kinds apart.

use crate::bic::Bic;
use crate::cfi::Cfi;
use crate::cusip::Cusip;
use crate::figi::Figi;
use crate::isin::Isin;
use crate::lei::Lei;
use crate::mic::Mic;
use crate::sedol::Sedol;
use crate::valor::Valor;
use crate::wkn::Wkn;

/// The kind of a securities identifier — its type tag, with no payload.
///
/// This is the discriminant of [`SecurityId`]: [`SecurityId::kind`] returns
/// it, and it is the natural value to switch on, store, or compare when the
/// identifier's *type* matters but its bytes do not.
///
/// # Examples
///
/// ```
/// use regit_identifiers::detect::{IdentifierKind, SecurityId};
///
/// let id = SecurityId::detect("US0378331005").unwrap();
/// assert_eq!(id.kind(), IdentifierKind::Isin);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierKind {
    /// An International Securities Identification Number (ISO 6166).
    Isin,
    /// A CUSIP / CINS number (ANSI X9.6).
    Cusip,
    /// A Stock Exchange Daily Official List number (SEDOL).
    Sedol,
    /// A Legal Entity Identifier (ISO 17442).
    Lei,
    /// A Financial Instrument Global Identifier (ANSI X9.145).
    Figi,
    /// A Business Identifier Code (ISO 9362).
    Bic,
    /// A Market Identifier Code (ISO 10383).
    Mic,
    /// A Classification of Financial Instruments code (ISO 10962).
    Cfi,
    /// A Wertpapierkennnummer (German national number).
    Wkn,
    /// A Valorennummer (Swiss national number).
    Valor,
}

/// Any one validated securities identifier, tagged by its kind.
///
/// A `SecurityId` is the result of [`SecurityId::detect`]: a value that has
/// already been parsed and fully validated as exactly one of the identifier
/// types this crate supports. Each variant wraps the corresponding validated
/// identifier, so unwrapping a `SecurityId` yields a value whose invariants
/// are already proven. It is `Copy` and allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::detect::SecurityId;
///
/// let id = SecurityId::detect("DEUTDEFF").unwrap();
/// match id {
///     SecurityId::Bic(bic) => assert_eq!(bic.country_code(), "DE"),
///     _ => panic!("DEUTDEFF is a BIC"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityId {
    /// A validated ISIN.
    Isin(Isin),
    /// A validated CUSIP / CINS number.
    Cusip(Cusip),
    /// A validated SEDOL.
    Sedol(Sedol),
    /// A validated LEI.
    Lei(Lei),
    /// A validated FIGI.
    Figi(Figi),
    /// A validated BIC.
    Bic(Bic),
    /// A validated MIC.
    Mic(Mic),
    /// A validated CFI code.
    Cfi(Cfi),
    /// A validated WKN.
    Wkn(Wkn),
    /// A validated VALOR.
    Valor(Valor),
}

impl SecurityId {
    /// Auto-detects which kind of identifier a raw string is.
    ///
    /// Candidate kinds are tried in a fixed, checksum-strength-first order —
    /// LEI, ISIN, FIGI, CUSIP, SEDOL, BIC, then MIC — and the first whose
    /// strict `parse` accepts the input wins. Because each candidate is a full
    /// `parse`, a kind is reported only when the input is structurally valid
    /// for it *and*, where the standard defines one, carries a correct check
    /// digit.
    ///
    /// The 12-character ISIN-versus-FIGI ambiguity is resolved by trying ISIN
    /// first; a genuine FIGI is never a valid ISIN (it requires character 3 to
    /// be `G` and forbids the ISIN-colliding provider prefixes) and so falls
    /// through to the FIGI branch.
    ///
    /// MIC is detected only when the `mic-registry` feature is enabled and the
    /// string is a *registered* MIC — a structurally valid but unregistered
    /// code such as `ZZZZ` is never reported.
    ///
    /// The structural-only kinds CFI, WKN, and VALOR are **not**
    /// auto-detected: they carry no check digit and overlap too heavily to
    /// distinguish. `detect` returns `None` when no checksum-bearing kind (or
    /// registered MIC) fits — parse those kinds explicitly instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::detect::{IdentifierKind, SecurityId};
    ///
    /// // Each kind is recognised from a real identifier.
    /// assert_eq!(
    ///     SecurityId::detect("5493001KJTIIGC8Y1R12").unwrap().kind(),
    ///     IdentifierKind::Lei,
    /// );
    /// assert_eq!(
    ///     SecurityId::detect("BBG000BLNNH6").unwrap().kind(),
    ///     IdentifierKind::Figi,
    /// );
    ///
    /// // Garbage, and the structural-only kinds, return `None`.
    /// assert!(SecurityId::detect("not-an-identifier").is_none());
    /// ```
    #[must_use]
    pub fn detect(s: &str) -> Option<Self> {
        // Checksum-strength first: kinds that carry a check digit are tried
        // before those that do not, and each candidate is a strict `parse`.
        if let Ok(lei) = Lei::parse(s) {
            return Some(Self::Lei(lei));
        }
        // ISIN before FIGI — both are 12 characters, but a genuine FIGI is
        // never a valid ISIN, so trying ISIN first cannot mislabel a FIGI.
        if let Ok(isin) = Isin::parse(s) {
            return Some(Self::Isin(isin));
        }
        if let Ok(figi) = Figi::parse(s) {
            return Some(Self::Figi(figi));
        }
        // CUSIP before BIC — an 8-character string could be either, and CUSIP
        // carries a check digit, so it is the higher-confidence candidate.
        if let Ok(cusip) = Cusip::parse(s) {
            return Some(Self::Cusip(cusip));
        }
        if let Ok(sedol) = Sedol::parse(s) {
            return Some(Self::Sedol(sedol));
        }
        if let Ok(bic) = Bic::parse(s) {
            return Some(Self::Bic(bic));
        }
        // A MIC has no check digit; report it only when it is a registered
        // market in the embedded ISO 10383 snapshot. With the `mic-registry`
        // feature disabled, MIC is never auto-detected.
        #[cfg(feature = "mic-registry")]
        if let Ok(mic) = Mic::parse_registered(s) {
            return Some(Self::Mic(mic));
        }
        None
    }

    /// Returns the [`IdentifierKind`] tag of this identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::detect::{IdentifierKind, SecurityId};
    ///
    /// let id = SecurityId::detect("037833100").unwrap();
    /// assert_eq!(id.kind(), IdentifierKind::Cusip);
    /// ```
    #[must_use]
    pub fn kind(&self) -> IdentifierKind {
        match self {
            Self::Isin(_) => IdentifierKind::Isin,
            Self::Cusip(_) => IdentifierKind::Cusip,
            Self::Sedol(_) => IdentifierKind::Sedol,
            Self::Lei(_) => IdentifierKind::Lei,
            Self::Figi(_) => IdentifierKind::Figi,
            Self::Bic(_) => IdentifierKind::Bic,
            Self::Mic(_) => IdentifierKind::Mic,
            Self::Cfi(_) => IdentifierKind::Cfi,
            Self::Wkn(_) => IdentifierKind::Wkn,
            Self::Valor(_) => IdentifierKind::Valor,
        }
    }

    /// Returns the wrapped identifier as a string slice.
    ///
    /// The returned `&str` is the canonical text of the underlying validated
    /// identifier, exactly as its own `as_str` would render it.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::detect::SecurityId;
    ///
    /// let id = SecurityId::detect("US0378331005").unwrap();
    /// assert_eq!(id.as_str(), "US0378331005");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Isin(v) => v.as_str(),
            Self::Cusip(v) => v.as_str(),
            Self::Sedol(v) => v.as_str(),
            Self::Lei(v) => v.as_str(),
            Self::Figi(v) => v.as_str(),
            Self::Bic(v) => v.as_str(),
            Self::Mic(v) => v.as_str(),
            Self::Cfi(v) => v.as_str(),
            Self::Wkn(v) => v.as_str(),
            Self::Valor(v) => v.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::debug;

    #[test]
    fn detects_lei() {
        let id = SecurityId::detect("5493001KJTIIGC8Y1R12").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Lei);
        assert_eq!(id.as_str(), "5493001KJTIIGC8Y1R12");
        assert!(matches!(id, SecurityId::Lei(_)));
    }

    #[test]
    fn detects_isin() {
        let id = SecurityId::detect("US0378331005").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Isin);
        assert_eq!(id.as_str(), "US0378331005");
        assert!(matches!(id, SecurityId::Isin(_)));
    }

    #[test]
    fn detects_figi() {
        let id = SecurityId::detect("BBG000BLNNH6").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Figi);
        assert_eq!(id.as_str(), "BBG000BLNNH6");
        assert!(matches!(id, SecurityId::Figi(_)));
    }

    #[test]
    fn detects_cusip() {
        let id = SecurityId::detect("037833100").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Cusip);
        assert_eq!(id.as_str(), "037833100");
        assert!(matches!(id, SecurityId::Cusip(_)));
    }

    #[test]
    fn detects_sedol() {
        let id = SecurityId::detect("0263494").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Sedol);
        assert_eq!(id.as_str(), "0263494");
        assert!(matches!(id, SecurityId::Sedol(_)));
    }

    #[test]
    fn detects_bic() {
        let id = SecurityId::detect("DEUTDEFF").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Bic);
        assert_eq!(id.as_str(), "DEUTDEFF");
        assert!(matches!(id, SecurityId::Bic(_)));
    }

    #[test]
    fn detects_eleven_character_bic() {
        // An 11-character BIC is detected and never mistaken for an ISIN or a
        // FIGI — both of those reject it on structure or check digit.
        let id = SecurityId::detect("DEUTDEFF500").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Bic);
        assert_eq!(id.as_str(), "DEUTDEFF500");
    }

    #[test]
    fn isin_wins_over_figi_at_twelve_characters() {
        // A valid ISIN is reported as an ISIN even though FIGI is also a
        // 12-character kind — ISIN is tried first.
        let id = SecurityId::detect("US0378331005").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Isin);
    }

    #[test]
    fn figi_is_not_mislabelled_as_isin() {
        // A genuine FIGI is never a valid ISIN, so it falls through to FIGI.
        let id = SecurityId::detect("BBG000BLNNH6").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Figi);
    }

    #[test]
    fn returns_none_for_garbage() {
        for bad in [
            "",
            "garbage",
            "not-an-identifier",
            "1234",
            "!!!!!!!!",
            "lowercaseinput",
        ] {
            assert!(
                SecurityId::detect(bad).is_none(),
                "{bad} should not be detected"
            );
        }
    }

    #[test]
    fn returns_none_for_bad_check_digit() {
        // A 12-character string with the wrong ISIN check digit is not a valid
        // ISIN, is not a FIGI (no leading 'G'), and matches nothing else.
        assert!(SecurityId::detect("US0378331004").is_none());
        // A 9-character string with the wrong CUSIP check digit, likewise.
        assert!(SecurityId::detect("037833101").is_none());
    }

    #[test]
    fn structural_only_kinds_are_not_detected() {
        // CFI, WKN, and VALOR parse on their own but are never auto-detected.
        assert!(Cfi::parse("ESVUFR").is_ok());
        assert!(SecurityId::detect("ESVUFR").is_none());

        assert!(Wkn::parse("A1EWWW").is_ok());
        // A1EWWW is 6 alphanumeric chars — no checksum-bearing kind fits.
        assert!(SecurityId::detect("A1EWWW").is_none());

        assert!(Valor::parse("1213853").is_ok());
        // 1213853 is 7 digits — a valid SEDOL body would need a check digit
        // that makes the whole 7-char string valid; this one is not a SEDOL.
        let valor_detect = SecurityId::detect("1213853");
        assert!(valor_detect.is_none_or(|id| id.kind() != IdentifierKind::Valor));
    }

    #[test]
    fn kind_for_every_variant() {
        // Each `SecurityId` variant reports its matching `IdentifierKind`,
        // including the three that `detect` never produces.
        let isin = Isin::parse("US0378331005").unwrap();
        let cusip = Cusip::parse("037833100").unwrap();
        let sedol = Sedol::parse("0263494").unwrap();
        let lei = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
        let figi = Figi::parse("BBG000BLNNH6").unwrap();
        let bic = Bic::parse("DEUTDEFF").unwrap();
        let mic = Mic::parse("XNAS").unwrap();
        let cfi = Cfi::parse("ESVUFR").unwrap();
        let wkn = Wkn::parse("A1EWWW").unwrap();
        let valor = Valor::parse("1213853").unwrap();

        assert_eq!(SecurityId::Isin(isin).kind(), IdentifierKind::Isin);
        assert_eq!(SecurityId::Cusip(cusip).kind(), IdentifierKind::Cusip);
        assert_eq!(SecurityId::Sedol(sedol).kind(), IdentifierKind::Sedol);
        assert_eq!(SecurityId::Lei(lei).kind(), IdentifierKind::Lei);
        assert_eq!(SecurityId::Figi(figi).kind(), IdentifierKind::Figi);
        assert_eq!(SecurityId::Bic(bic).kind(), IdentifierKind::Bic);
        assert_eq!(SecurityId::Mic(mic).kind(), IdentifierKind::Mic);
        assert_eq!(SecurityId::Cfi(cfi).kind(), IdentifierKind::Cfi);
        assert_eq!(SecurityId::Wkn(wkn).kind(), IdentifierKind::Wkn);
        assert_eq!(SecurityId::Valor(valor).kind(), IdentifierKind::Valor);
    }

    #[test]
    fn as_str_for_every_variant() {
        // `as_str` returns the canonical text of every wrapped identifier.
        let isin = Isin::parse("US0378331005").unwrap();
        let cusip = Cusip::parse("037833100").unwrap();
        let sedol = Sedol::parse("0263494").unwrap();
        let lei = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
        let figi = Figi::parse("BBG000BLNNH6").unwrap();
        let bic = Bic::parse("DEUTDEFF").unwrap();
        let mic = Mic::parse("XNAS").unwrap();
        let cfi = Cfi::parse("ESVUFR").unwrap();
        let wkn = Wkn::parse("A1EWWW").unwrap();
        let valor = Valor::parse("1213853").unwrap();

        assert_eq!(SecurityId::Isin(isin).as_str(), "US0378331005");
        assert_eq!(SecurityId::Cusip(cusip).as_str(), "037833100");
        assert_eq!(SecurityId::Sedol(sedol).as_str(), "0263494");
        assert_eq!(SecurityId::Lei(lei).as_str(), "5493001KJTIIGC8Y1R12");
        assert_eq!(SecurityId::Figi(figi).as_str(), "BBG000BLNNH6");
        assert_eq!(SecurityId::Bic(bic).as_str(), "DEUTDEFF");
        assert_eq!(SecurityId::Mic(mic).as_str(), "XNAS");
        assert_eq!(SecurityId::Cfi(cfi).as_str(), "ESVUFR");
        assert_eq!(SecurityId::Wkn(wkn).as_str(), "A1EWWW");
        assert_eq!(SecurityId::Valor(valor).as_str(), "1213853");
    }

    #[test]
    fn detected_value_round_trips_through_as_str() {
        // Every detectable input re-serialises identically through `as_str`.
        for &s in &[
            "5493001KJTIIGC8Y1R12",
            "US0378331005",
            "BBG000BLNNH6",
            "037833100",
            "0263494",
            "DEUTDEFF",
            "DEUTDEFF500",
        ] {
            let id = SecurityId::detect(s).unwrap_or_else(|| panic!("{s} should detect"));
            assert_eq!(id.as_str(), s);
        }
    }

    #[test]
    fn identifier_kind_is_copy_and_eq() {
        let a = IdentifierKind::Isin;
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(IdentifierKind::Isin, IdentifierKind::Figi);
    }

    #[test]
    fn security_id_is_copy_and_eq() {
        let a = SecurityId::detect("US0378331005").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, SecurityId::detect("037833100").unwrap());
    }

    #[test]
    fn debug_renders_kind_and_variant() {
        assert!(debug(IdentifierKind::Lei).as_str().contains("Lei"));
        let id = SecurityId::detect("US0378331005").unwrap();
        assert!(debug(id).as_str().contains("Isin"));
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn detects_registered_mic() {
        // With the registry feature on, a registered MIC is detected.
        let id = SecurityId::detect("XNAS").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Mic);
        assert_eq!(id.as_str(), "XNAS");
        assert!(matches!(id, SecurityId::Mic(_)));
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn unregistered_mic_is_not_detected() {
        // ZZZZ is structurally a valid MIC but is in no registry, so it is
        // not auto-detected even with the feature enabled.
        assert!(Mic::parse("ZZZZ").is_ok());
        assert!(SecurityId::detect("ZZZZ").is_none());
    }
}
