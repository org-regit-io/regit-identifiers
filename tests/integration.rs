// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for regit-identifiers.
//!
//! Structure (see `doc/WORKING.md` §5):
//!   - mod golden      -- real instruments parse and round-trip
//!   - mod invalid     -- documented bad cases rejected with the right error
//!   - mod roundtrip   -- `parse(x).as_str()` == x for every kind
//!   - mod convert     -- ISIN <-> CUSIP / SEDOL / VALOR conversion identities
//!   - mod detect      -- `SecurityId::detect` across every detectable kind
//!   - mod registry    -- embedded ISO 10383 MIC registry (mic-registry only)
//!   - mod properties  -- proptest invariants

use regit_identifiers::checkdigit;
use regit_identifiers::convert::{
    build_isin, cusip_to_isin, isin_to_cusip, isin_to_sedol, isin_to_valor, sedol_to_isin,
    valor_to_isin,
};
use regit_identifiers::detect::{IdentifierKind, SecurityId};
use regit_identifiers::errors::ValidationError;
use regit_identifiers::{Bic, Cfi, Cusip, Figi, Isin, Lei, Mic, Sedol, Valor, Wkn};

// ─── Golden vectors ──────────────────────────────────────────────────────────

mod golden {
    use super::*;

    #[test]
    fn isin_real_instruments() {
        for &s in &[
            "US0378331005", // Apple Inc.
            "US5949181045", // Microsoft Corp.
            "GB0002634946", // BAE Systems plc
            "DE000BAY0017", // Bayer AG
            "FR0000131104", // BNP Paribas
            "NL0011794037", // ABN AMRO
        ] {
            let isin = Isin::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(isin.as_str(), s);
        }
    }

    #[test]
    fn isin_segments_for_apple() {
        let isin = Isin::parse("US0378331005").unwrap();
        assert_eq!(isin.country_code(), "US");
        assert_eq!(isin.nsin(), "037833100");
        assert_eq!(isin.check_digit(), '5');
    }

    #[test]
    fn cusip_real_instruments() {
        for &s in &["037833100", "594918104", "38259P508"] {
            let cusip = Cusip::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(cusip.as_str(), s);
        }
        let apple = Cusip::parse("037833100").unwrap();
        assert_eq!(apple.issuer(), "037833");
        assert_eq!(apple.issue(), "10");
        assert!(!apple.is_cins());
    }

    #[test]
    fn sedol_real_instruments() {
        let bae = Sedol::parse("0263494").unwrap();
        assert_eq!(bae.body(), "026349");
        assert!(bae.is_legacy_numeric());
        // A post-2004 alphanumeric SEDOL.
        let modern = Sedol::parse("B0WNLY7").unwrap();
        assert!(!modern.is_legacy_numeric());
    }

    #[test]
    fn lei_real_instruments() {
        let bloomberg = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
        assert_eq!(bloomberg.lou_prefix(), "5493");
        assert_eq!(bloomberg.entity_id(), "1KJTIIGC8Y1R");
        assert_eq!(bloomberg.check_digits(), "12");
        assert!(Lei::parse("549300DTUYXVMJXZNY75").is_ok());
    }

    #[test]
    fn figi_real_instruments() {
        for &s in &["BBG000BLNNH6", "BBG000B9XRY4", "BBG000BVPV84"] {
            let figi = Figi::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(figi.as_str(), s);
            assert!(figi.is_bloomberg());
        }
        let ibm = Figi::parse("BBG000BLNNH6").unwrap();
        assert_eq!(ibm.provider_prefix(), "BBG");
        assert_eq!(ibm.body(), "000BLNNH");
    }

    #[test]
    fn bic_real_institutions() {
        for &s in &[
            "DEUTDEFF",
            "DEUTDEFF500",
            "CHASUS33",
            "BOFAUS3N",
            "NDEAFIHH",
        ] {
            let bic = Bic::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(bic.as_str(), s);
        }
        let db = Bic::parse("DEUTDEFF500").unwrap();
        assert_eq!(db.institution(), "DEUT");
        assert_eq!(db.country_code(), "DE");
        assert_eq!(db.branch_code(), Some("500"));
        assert!(db.has_branch());
    }

    #[test]
    fn mic_real_markets() {
        for &s in &["XNAS", "XLON", "XPAR", "XNYS"] {
            let mic = Mic::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(mic.as_str(), s);
        }
        assert_eq!(Mic::parse("XNAS").unwrap().suffix(), "NAS");
    }

    #[test]
    fn cfi_category_letters() {
        let equity = Cfi::parse("ESVUFR").unwrap();
        assert_eq!(equity.category(), 'E');
        assert_eq!(equity.category_name(), "Equities");
        let debt = Cfi::parse("DBFUGR").unwrap();
        assert_eq!(debt.category_name(), "Debt instruments");
    }

    #[test]
    fn wkn_real_instruments() {
        assert!(Wkn::parse("766403").unwrap().is_numeric()); // Volkswagen AG
        assert!(Wkn::parse("519000").unwrap().is_numeric()); // BMW AG
        assert!(!Wkn::parse("A1EWWW").unwrap().is_numeric()); // Adidas AG
    }

    #[test]
    fn valor_real_instruments() {
        let valor = Valor::parse("1213853").unwrap();
        assert_eq!(valor.len(), 7);
        assert_eq!(valor.as_u64(), 1_213_853);
        assert!(Valor::parse("908440").is_ok());
        assert!(Valor::parse("24476758").is_ok());
    }

    #[test]
    fn checkdigit_worked_examples() {
        // Hand-computed against doc/ALGORITHMS.md.
        assert_eq!(checkdigit::luhn_checksum("7992739871").unwrap(), 3);
        assert_eq!(checkdigit::isin_check_digit("US037833100").unwrap(), '5');
        assert_eq!(checkdigit::cusip_check_digit("03783310").unwrap(), '0');
        assert_eq!(checkdigit::sedol_check_digit("026349").unwrap(), '4');
        assert_eq!(checkdigit::figi_check_digit("BBG000BLNNH").unwrap(), '6');
        assert_eq!(
            checkdigit::lei_check_digits("5493001KJTIIGC8Y1R").unwrap(),
            ['1', '2'],
        );
    }
}

// ─── Invalid vectors ─────────────────────────────────────────────────────────

mod invalid {
    use super::*;

    #[test]
    fn isin_wrong_check_digit() {
        assert_eq!(
            Isin::parse("US0378331004"),
            Err(ValidationError::BadCheckDigit {
                expected: '5',
                found: '4',
            }),
        );
    }

    #[test]
    fn isin_wrong_length() {
        assert_eq!(
            Isin::parse("US037833100"),
            Err(ValidationError::WrongLength {
                expected: 12,
                found: 11,
            }),
        );
    }

    #[test]
    fn isin_unknown_country_code() {
        assert_eq!(
            Isin::parse("ZZ0378331005"),
            Err(ValidationError::InvalidCountryCode),
        );
    }

    #[test]
    fn isin_lower_case_is_an_invalid_character() {
        assert!(matches!(
            Isin::parse("us0378331005"),
            Err(ValidationError::InvalidCharacter { position: 1, .. }),
        ));
    }

    #[test]
    fn cusip_wrong_check_digit() {
        assert_eq!(
            Cusip::parse("037833101"),
            Err(ValidationError::BadCheckDigit {
                expected: '0',
                found: '1',
            }),
        );
    }

    #[test]
    fn cusip_illegal_body_character() {
        assert!(matches!(
            Cusip::parse("0378/3100"),
            Err(ValidationError::InvalidCharacter { position: 5, .. }),
        ));
    }

    #[test]
    fn sedol_wrong_check_digit() {
        assert_eq!(
            Sedol::parse("0263495"),
            Err(ValidationError::BadCheckDigit {
                expected: '4',
                found: '5',
            }),
        );
    }

    #[test]
    fn sedol_vowel_in_body_is_rejected() {
        assert_eq!(
            Sedol::parse("B0WNLA7"),
            Err(ValidationError::InvalidCharacter {
                position: 6,
                found: 'A',
            }),
        );
    }

    #[test]
    fn lei_wrong_check_digit() {
        assert_eq!(
            Lei::parse("5493001KJTIIGC8Y1R13"),
            Err(ValidationError::BadCheckDigit {
                expected: '2',
                found: '3',
            }),
        );
    }

    #[test]
    fn lei_reserved_field_must_be_zero_zero() {
        assert_eq!(
            Lei::parse("5493011KJTIIGC8Y1R12"),
            Err(ValidationError::Structure {
                rule: "LEI positions 5-6 must be 00",
            }),
        );
    }

    #[test]
    fn figi_wrong_check_digit() {
        assert_eq!(
            Figi::parse("BBG000BLNNH5"),
            Err(ValidationError::BadCheckDigit {
                expected: '6',
                found: '5',
            }),
        );
    }

    #[test]
    fn figi_forbidden_prefix_is_rejected() {
        assert_eq!(
            Figi::parse("BSG000BLNNH6"),
            Err(ValidationError::Structure {
                rule: "FIGI provider prefix must not be an ISIN country code",
            }),
        );
    }

    #[test]
    fn bic_wrong_length_is_a_structural_error() {
        assert_eq!(
            Bic::parse("DEUTDEFF5"),
            Err(ValidationError::Structure {
                rule: "BIC length must be 8 or 11",
            }),
        );
    }

    #[test]
    fn bic_unknown_country_code() {
        assert_eq!(
            Bic::parse("DEUTZZFF"),
            Err(ValidationError::InvalidCountryCode),
        );
    }

    #[test]
    fn mic_wrong_length() {
        assert_eq!(
            Mic::parse("XNA"),
            Err(ValidationError::WrongLength {
                expected: 4,
                found: 3,
            }),
        );
    }

    #[test]
    fn mic_leading_digit_is_rejected() {
        assert_eq!(
            Mic::parse("1NAS"),
            Err(ValidationError::InvalidCharacter {
                position: 1,
                found: '1',
            }),
        );
    }

    #[test]
    fn cfi_unknown_category_letter() {
        assert_eq!(
            Cfi::parse("QSVUFR"),
            Err(ValidationError::Structure {
                rule: "CFI category must be one of E C D R O F S H I J K L T M",
            }),
        );
    }

    #[test]
    fn wkn_letters_i_and_o_are_rejected() {
        assert_eq!(
            Wkn::parse("A1IWWW"),
            Err(ValidationError::InvalidCharacter {
                position: 3,
                found: 'I',
            }),
        );
        assert_eq!(
            Wkn::parse("A1OWWW"),
            Err(ValidationError::InvalidCharacter {
                position: 3,
                found: 'O',
            }),
        );
    }

    #[test]
    fn valor_length_bounds_are_structural() {
        assert_eq!(
            Valor::parse(""),
            Err(ValidationError::Structure {
                rule: "VALOR must be 1 to 9 digits",
            }),
        );
        assert_eq!(
            Valor::parse("1234567890"),
            Err(ValidationError::Structure {
                rule: "VALOR must be 1 to 9 digits",
            }),
        );
    }

    #[test]
    fn valor_non_digit_is_rejected() {
        assert_eq!(
            Valor::parse("1213853A"),
            Err(ValidationError::InvalidCharacter {
                position: 8,
                found: 'A',
            }),
        );
    }

    #[test]
    fn non_ascii_input_never_panics_any_parser() {
        // A multi-byte character must be rejected cleanly by every parser.
        assert!(Isin::parse("US037833100é").is_err());
        assert!(Cusip::parse("03783310é").is_err());
        assert!(Sedol::parse("026349é").is_err());
        assert!(Lei::parse("5493001KJTIIGC8Y1Ré2").is_err());
        assert!(Figi::parse("BBG000BLNNHé").is_err());
        assert!(Bic::parse("DEUTDEFé").is_err());
        assert!(Mic::parse("XNAé").is_err());
        assert!(Cfi::parse("ESVUFÉ").is_err());
        assert!(Wkn::parse("A1EWWé").is_err());
        assert!(Valor::parse("12345é").is_err());
    }
}

// ─── Round-trips ─────────────────────────────────────────────────────────────

mod roundtrip {
    use super::*;

    /// `parse(x).as_str() == x` must hold for every kind and every input.
    #[test]
    fn parse_then_as_str_is_identity() {
        for &s in &["US0378331005", "GB0002634946", "DE000BAY0017"] {
            assert_eq!(Isin::parse(s).unwrap().as_str(), s);
        }
        for &s in &["037833100", "594918104", "38259P508"] {
            assert_eq!(Cusip::parse(s).unwrap().as_str(), s);
        }
        for &s in &["0263494", "0540528", "B0WNLY7"] {
            assert_eq!(Sedol::parse(s).unwrap().as_str(), s);
        }
        for &s in &["5493001KJTIIGC8Y1R12", "549300DTUYXVMJXZNY75"] {
            assert_eq!(Lei::parse(s).unwrap().as_str(), s);
        }
        for &s in &["BBG000BLNNH6", "BBG000B9XRY4", "BBG000BVPV84"] {
            assert_eq!(Figi::parse(s).unwrap().as_str(), s);
        }
        for &s in &["DEUTDEFF", "DEUTDEFF500", "CHASUS33"] {
            assert_eq!(Bic::parse(s).unwrap().as_str(), s);
        }
        for &s in &["XNAS", "XLON", "XPAR"] {
            assert_eq!(Mic::parse(s).unwrap().as_str(), s);
        }
        for &s in &["ESVUFR", "DBFUGR", "OCASPS"] {
            assert_eq!(Cfi::parse(s).unwrap().as_str(), s);
        }
        for &s in &["766403", "519000", "A1EWWW"] {
            assert_eq!(Wkn::parse(s).unwrap().as_str(), s);
        }
        for &s in &["1213853", "908440", "24476758", "7", "123456789"] {
            assert_eq!(Valor::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_bytes_unchecked_matches_parse() {
        assert_eq!(
            Isin::from_bytes_unchecked(*b"US0378331005"),
            Isin::parse("US0378331005").unwrap(),
        );
        assert_eq!(
            Cusip::from_bytes_unchecked(*b"037833100"),
            Cusip::parse("037833100").unwrap(),
        );
        assert_eq!(
            Bic::from_bytes_unchecked(*b"DEUTDEFF\0\0\0", 8),
            Bic::parse("DEUTDEFF").unwrap(),
        );
        assert_eq!(
            Valor::from_bytes_unchecked(*b"908440\0\0\0", 6),
            Valor::parse("908440").unwrap(),
        );
    }
}

// ─── Conversions ─────────────────────────────────────────────────────────────

mod convert {
    use super::*;

    #[test]
    fn isin_cusip_round_trip() {
        for &s in &["US0378331005", "US5949181045"] {
            let isin = Isin::parse(s).unwrap();
            let cusip = isin_to_cusip(&isin).unwrap();
            let back = cusip_to_isin(&cusip, isin.country_code()).unwrap();
            assert_eq!(back, isin);
        }
    }

    #[test]
    fn cusip_isin_round_trip() {
        for &s in &["037833100", "594918104", "38259P508"] {
            let cusip = Cusip::parse(s).unwrap();
            let isin = cusip_to_isin(&cusip, "US").unwrap();
            assert_eq!(isin_to_cusip(&isin).unwrap(), cusip);
        }
    }

    #[test]
    fn isin_sedol_round_trip() {
        let isin = Isin::parse("GB0002634946").unwrap();
        let sedol = isin_to_sedol(&isin).unwrap();
        assert_eq!(sedol.as_str(), "0263494");
        assert_eq!(sedol_to_isin(&sedol, isin.country_code()).unwrap(), isin);
    }

    #[test]
    fn sedol_isin_round_trip() {
        for &s in &["0263494", "0540528", "B0WNLY7"] {
            let sedol = Sedol::parse(s).unwrap();
            let isin = sedol_to_isin(&sedol, "GB").unwrap();
            assert_eq!(isin_to_sedol(&isin).unwrap(), sedol);
        }
    }

    #[test]
    fn isin_valor_round_trip() {
        let isin = Isin::parse("CH0012138530").unwrap();
        let valor = isin_to_valor(&isin).unwrap();
        assert_eq!(valor.as_str(), "1213853");
        assert_eq!(valor_to_isin(&valor, isin.country_code()).unwrap(), isin);
    }

    #[test]
    fn valor_isin_round_trip() {
        for &v in &["1213853", "908440", "24476758", "123456789", "7"] {
            let valor = Valor::parse(v).unwrap();
            let isin = valor_to_isin(&valor, "CH").unwrap();
            assert_eq!(isin_to_valor(&isin).unwrap(), valor);
        }
    }

    #[test]
    fn build_isin_agrees_with_isin_parse() {
        for &(country, nsin, expected) in &[
            ("US", "037833100", "US0378331005"),
            ("US", "594918104", "US5949181045"),
            ("GB", "000263494", "GB0002634946"),
            ("DE", "000BAY001", "DE000BAY0017"),
            ("CH", "001213853", "CH0012138530"),
        ] {
            let built = build_isin(country, nsin).unwrap();
            assert_eq!(built.as_str(), expected);
            assert_eq!(built, Isin::parse(expected).unwrap());
        }
    }

    #[test]
    fn check_digit_schemes_are_independent() {
        // The CUSIP check digit and the ISIN check digit are computed by
        // unrelated algorithms; conversion recomputes the target's digit.
        let cusip = Cusip::parse("037833100").unwrap();
        assert_eq!(cusip.check_digit(), '0');
        assert_eq!(cusip_to_isin(&cusip, "US").unwrap().check_digit(), '5');
    }

    #[test]
    fn conversion_rejects_unsupported_country() {
        use regit_identifiers::errors::ConversionError;
        // A GB ISIN has no embedded CUSIP.
        let gb = Isin::parse("GB0002634946").unwrap();
        assert_eq!(isin_to_cusip(&gb), Err(ConversionError::UnsupportedCountry));
        // A US ISIN embeds no SEDOL and no VALOR.
        let us = Isin::parse("US0378331005").unwrap();
        assert_eq!(isin_to_sedol(&us), Err(ConversionError::UnsupportedCountry));
        assert_eq!(isin_to_valor(&us), Err(ConversionError::UnsupportedCountry));
    }
}

// ─── Auto-detection ──────────────────────────────────────────────────────────

mod detect {
    use super::*;

    #[test]
    fn detects_every_checksum_bearing_kind() {
        for &(s, kind) in &[
            ("5493001KJTIIGC8Y1R12", IdentifierKind::Lei),
            ("US0378331005", IdentifierKind::Isin),
            ("BBG000BLNNH6", IdentifierKind::Figi),
            ("037833100", IdentifierKind::Cusip),
            ("0263494", IdentifierKind::Sedol),
            ("DEUTDEFF", IdentifierKind::Bic),
            ("DEUTDEFF500", IdentifierKind::Bic),
        ] {
            let id = SecurityId::detect(s).unwrap_or_else(|| panic!("{s} should detect"));
            assert_eq!(id.kind(), kind, "for input {s}");
            assert_eq!(id.as_str(), s);
        }
    }

    #[test]
    fn isin_wins_over_figi_at_twelve_characters() {
        let id = SecurityId::detect("US0378331005").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Isin);
    }

    #[test]
    fn figi_is_not_mislabelled_as_isin() {
        let id = SecurityId::detect("BBG000BLNNH6").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Figi);
    }

    #[test]
    fn garbage_and_bad_check_digits_return_none() {
        for bad in [
            "",
            "garbage",
            "not-an-identifier",
            "US0378331004", // wrong ISIN check digit
            "037833101",    // wrong CUSIP check digit
        ] {
            assert!(SecurityId::detect(bad).is_none(), "{bad} should not detect");
        }
    }

    #[test]
    fn structural_only_kinds_are_not_detected() {
        // CFI, WKN, and VALOR parse on their own but carry no check digit, so
        // detection never reports them.
        assert!(Cfi::parse("ESVUFR").is_ok());
        assert!(SecurityId::detect("ESVUFR").is_none());
        assert!(Wkn::parse("A1EWWW").is_ok());
        assert!(SecurityId::detect("A1EWWW").is_none());
    }
}

// ─── MIC registry (mic-registry feature only) ────────────────────────────────

#[cfg(feature = "mic-registry")]
mod registry {
    use super::*;
    use regit_identifiers::mic_registry::{self, MicStatus, SNAPSHOT_DATE};

    #[test]
    fn snapshot_date_is_iso_formatted() {
        // YYYY-MM-DD — ten characters with dashes at indices 4 and 7.
        assert_eq!(SNAPSHOT_DATE.len(), 10);
        let bytes = SNAPSHOT_DATE.as_bytes();
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
    }

    #[test]
    fn registry_is_sorted_by_mic() {
        // The lookup is a binary search, so the table must stay sorted.
        let mut previous = "";
        for entry in mic_registry::REGISTRY {
            assert!(entry.mic > previous, "{} follows {previous}", entry.mic);
            previous = entry.mic;
        }
    }

    #[test]
    fn lookup_finds_real_markets() {
        for &code in &["XNAS", "XLON", "XPAR", "XNYS"] {
            let entry =
                mic_registry::lookup(code).unwrap_or_else(|| panic!("{code} should be registered"));
            assert_eq!(entry.mic, code);
        }
    }

    #[test]
    fn lookup_misses_unregistered_code() {
        assert!(mic_registry::lookup("ZZZZ").is_none());
    }

    #[test]
    fn nasdaq_metadata_is_as_published() {
        let nasdaq = mic_registry::lookup("XNAS").expect("XNAS is registered");
        assert_eq!(nasdaq.country, "US");
        assert!(nasdaq.is_operating);
        assert_eq!(nasdaq.status, MicStatus::Active);
        assert_eq!(nasdaq.operating_mic, "XNAS");
    }

    #[test]
    fn mic_lookup_and_parse_registered_agree() {
        let mic = Mic::parse("XLON").unwrap();
        assert!(mic.is_registered());
        assert!(mic.lookup().is_some());
        assert!(Mic::parse_registered("XLON").is_ok());

        // Well-formed but unregistered.
        assert_eq!(
            Mic::parse_registered("ZZZZ"),
            Err(ValidationError::Structure {
                rule: "MIC is not in the ISO 10383 registry",
            }),
        );
    }

    #[test]
    fn detect_reports_registered_mic() {
        let id = SecurityId::detect("XNAS").unwrap();
        assert_eq!(id.kind(), IdentifierKind::Mic);
        // An unregistered but well-formed MIC is not detected.
        assert!(SecurityId::detect("ZZZZ").is_none());
    }
}

// ─── proptest invariants ─────────────────────────────────────────────────────

mod properties {
    use super::*;
    use proptest::prelude::*;

    /// Every parser used by the "never panic" property, keyed by name.
    fn parse_with_every_kind(s: &str) {
        // None of these may panic, whatever the input bytes are.
        let _ = Isin::parse(s);
        let _ = Cusip::parse(s);
        let _ = Sedol::parse(s);
        let _ = Lei::parse(s);
        let _ = Figi::parse(s);
        let _ = Bic::parse(s);
        let _ = Mic::parse(s);
        let _ = Cfi::parse(s);
        let _ = Wkn::parse(s);
        let _ = Valor::parse(s);
        let _ = SecurityId::detect(s);
    }

    proptest! {
        /// A random string — any bytes, any length — never panics a parser.
        #[test]
        fn random_strings_never_panic_parse(s in ".{0,40}") {
            parse_with_every_kind(&s);
        }

        /// Random bytes restricted to the identifier alphabet likewise never
        /// panic, and exercise the structural and check-digit paths harder.
        #[test]
        fn random_alphanumeric_never_panics(s in "[A-Z0-9*@#]{0,24}") {
            parse_with_every_kind(&s);
        }

        /// Whatever `Isin::parse` accepts re-serialises identically.
        #[test]
        fn accepted_isin_round_trips(s in "[A-Z0-9]{12}") {
            if let Ok(isin) = Isin::parse(&s) {
                prop_assert_eq!(isin.as_str(), s.as_str());
            }
        }

        /// Whatever `Cusip::parse` accepts re-serialises identically.
        #[test]
        fn accepted_cusip_round_trips(s in "[A-Z0-9*@#]{9}") {
            if let Ok(cusip) = Cusip::parse(&s) {
                prop_assert_eq!(cusip.as_str(), s.as_str());
            }
        }

        /// Whatever `SecurityId::detect` accepts re-serialises identically.
        #[test]
        fn detected_value_round_trips(s in "[A-Z0-9]{4,20}") {
            if let Some(id) = SecurityId::detect(&s) {
                prop_assert_eq!(id.as_str(), s.as_str());
            }
        }

        /// A random valid CUSIP body plus its computed check digit always
        /// parses — the parser recomputes the same digit and accepts it.
        #[test]
        fn cusip_body_plus_check_always_parses(body in "[A-Z0-9]{8}") {
            let check = checkdigit::cusip_check_digit(&body).expect("8-char body");
            let full = alloc_push(&body, check);
            let cusip = Cusip::parse(&full).expect("body + computed check must parse");
            prop_assert_eq!(cusip.check_digit(), check);
        }

        /// A random valid SEDOL body plus its computed check digit always
        /// parses. The body strategy excludes vowels, which SEDOL forbids.
        #[test]
        fn sedol_body_plus_check_always_parses(body in "[B-DF-HJ-NP-TV-Z0-9]{6}") {
            let check = checkdigit::sedol_check_digit(&body).expect("6-char body");
            let full = alloc_push(&body, check);
            let sedol = Sedol::parse(&full).expect("body + computed check must parse");
            prop_assert_eq!(sedol.check_digit(), check);
        }

        /// A random valid ISIN body plus its computed check digit always
        /// parses, provided the body opens with a real country prefix.
        #[test]
        fn isin_body_plus_check_always_parses(nsin in "[A-Z0-9]{9}") {
            let body = alloc_concat("US", &nsin);
            let check = checkdigit::isin_check_digit(&body).expect("11-char body");
            let full = alloc_push(&body, check);
            let isin = Isin::parse(&full).expect("US body + computed check must parse");
            prop_assert_eq!(isin.check_digit(), check);
        }
    }

    /// Appends a single character to a string. A test-only `std` helper —
    /// these integration tests are a normal `std` binary, not the `no_std`
    /// library.
    fn alloc_push(s: &str, ch: char) -> String {
        let mut out = String::with_capacity(s.len() + 1);
        out.push_str(s);
        out.push(ch);
        out
    }

    /// Concatenates two string slices.
    fn alloc_concat(a: &str, b: &str) -> String {
        let mut out = String::with_capacity(a.len() + b.len());
        out.push_str(a);
        out.push_str(b);
        out
    }
}
