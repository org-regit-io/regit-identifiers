// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Conversions between securities identifiers.
//!
//! An ISIN is the international wrapper around a *national* securities number
//! (the NSIN). For three jurisdictions that wrapper is exact and reversible:
//!
//! ```text
//!   US / CA ISIN   country prefix · 9-char NSIN · check   NSIN  IS  the CUSIP
//!   GB / IE ISIN   country prefix · 00 + SEDOL  · check   NSIN  IS  00 + SEDOL
//!   CH / LI ISIN   country prefix · 0…0 + VALOR · check   NSIN  IS  padded VALOR
//! ```
//!
//! - A **US** or **CA** ISIN embeds a [`Cusip`] verbatim as its nine-character
//!   NSIN. Extracting the CUSIP is taking those nine characters; building the
//!   ISIN is prefixing `US` or `CA` and computing a fresh ISIN check digit.
//! - A **GB** or **IE** ISIN embeds a seven-character [`Sedol`] right-aligned
//!   in the nine-character NSIN, left-padded with the two literal characters
//!   `00`. Extracting the SEDOL strips that `00`.
//! - A **CH** or **LI** ISIN embeds a [`Valor`] — a one-to-nine-digit number —
//!   left-padded with zeros to nine digits. Extracting the VALOR strips the
//!   leading zeros.
//!
//! The two check-digit schemes are **independent**: the CUSIP, SEDOL, and ISIN
//! algorithms are unrelated, so every conversion recomputes — never reuses —
//! the target's check digit and re-parses the result through the target type's
//! own validator. A conversion with no defined meaning (a non-US/CA ISIN to a
//! CUSIP, say) returns [`ConversionError::UnsupportedCountry`]; a conversion
//! whose result is not a valid identifier returns
//! [`ConversionError::NotConvertible`] or [`ConversionError::Validation`]. A
//! conversion never returns a wrong answer.
//!
//! Every `*_to_isin` function and [`build_isin`] left-pads a short national
//! number into the nine-character NSIN field, so a [`Sedol`] (`00` + 7) and a
//! [`Valor`] (zero-padded to 9) land where the standard places them.
//!
//! # References
//!
//! - ISO 6166 (ISIN), ANSI X9.6 (CUSIP), London Stock Exchange (SEDOL),
//!   SIX Financial Information (VALOR) — the schemes whose embedding rules
//!   these conversions implement.

use crate::checkdigit;
use crate::country;
use crate::cusip::Cusip;
use crate::errors::ConversionError;
use crate::isin::Isin;
use crate::sedol::Sedol;
use crate::valor::Valor;

/// Length of an ISIN's nine-character NSIN field.
const NSIN_LEN: usize = 9;

/// Length of an ISIN's eleven-character body (country prefix + NSIN).
const BODY_LEN: usize = 11;

/// Assembles an ISIN from a two-character country prefix and a national
/// number, computing the ISIN check digit.
///
/// The national number is left-zero-padded into the nine-character NSIN
/// field, the eleven-character body is formed, the ISIN check digit is
/// computed from that body via [`checkdigit::isin_check_digit`], and the
/// resulting twelve characters are re-parsed through [`Isin::parse`] so the
/// returned value is always a fully validated ISIN.
///
/// # Errors
///
/// - [`ConversionError::UnsupportedCountry`] if `country` is not a recognised
///   ISIN prefix (an ISO 3166-1 code or an ISIN substitute prefix).
/// - [`ConversionError::NotConvertible`] if `country` is not exactly two
///   characters, or if `nsin` is empty or longer than nine characters.
/// - [`ConversionError::Validation`] if the assembled string is not a valid
///   ISIN — for instance because `nsin` contains a character outside
///   `[A-Z0-9]`.
///
/// # Examples
///
/// ```
/// use regit_identifiers::convert::build_isin;
///
/// // Apple's CUSIP, wrapped into its ISIN with a fresh check digit.
/// let isin = build_isin("US", "037833100").unwrap();
/// assert_eq!(isin.as_str(), "US0378331005");
/// ```
pub fn build_isin(country: &str, nsin: &str) -> Result<Isin, ConversionError> {
    // The country prefix must be exactly two ASCII characters and a
    // recognised ISIN prefix.
    if country.len() != 2 || !country.is_ascii() {
        return Err(ConversionError::NotConvertible {
            reason: "country prefix must be exactly two characters",
        });
    }
    if !country::is_isin_prefix(country) {
        return Err(ConversionError::UnsupportedCountry);
    }
    // The national number must be ASCII and fit, left-padded, into the
    // nine-character NSIN field.
    if !nsin.is_ascii() {
        return Err(ConversionError::NotConvertible {
            reason: "national number must be ASCII",
        });
    }
    let nsin_bytes = nsin.as_bytes();
    if nsin_bytes.is_empty() || nsin_bytes.len() > NSIN_LEN {
        return Err(ConversionError::NotConvertible {
            reason: "national number must be 1 to 9 characters",
        });
    }
    // Build the eleven-character body: country prefix, then the national
    // number right-aligned in nine characters with leading '0' padding.
    let mut body = [b'0'; BODY_LEN];
    body[0] = country.as_bytes()[0];
    body[1] = country.as_bytes()[1];
    let pad = NSIN_LEN - nsin_bytes.len();
    if let Some(slot) = body.get_mut(2 + pad..BODY_LEN) {
        slot.copy_from_slice(nsin_bytes);
    }
    let body_str = core::str::from_utf8(&body).unwrap_or("");
    // Compute the ISIN check digit from the assembled body.
    let check = checkdigit::isin_check_digit(body_str)?;
    // Form the twelve-character ISIN and re-parse it for full validation.
    let mut full = [0u8; Isin::LENGTH];
    if let Some(slot) = full.get_mut(0..BODY_LEN) {
        slot.copy_from_slice(&body);
    }
    full[BODY_LEN] = check as u8;
    let full_str = core::str::from_utf8(&full).unwrap_or("");
    Ok(Isin::parse(full_str)?)
}

/// Extracts the [`Cusip`] embedded in a United States or Canada ISIN.
///
/// For a `US` or `CA` ISIN the nine-character NSIN *is* the CUSIP. The nine
/// characters are re-parsed through [`Cusip::parse`], so the CUSIP's own check
/// digit — computed by a different algorithm than the ISIN's — is verified.
///
/// # Errors
///
/// - [`ConversionError::UnsupportedCountry`] if the ISIN's country prefix is
///   not `US` or `CA`; only those jurisdictions use a CUSIP as their NSIN.
/// - [`ConversionError::Validation`] if the nine-character NSIN is not itself
///   a valid CUSIP.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Isin;
/// use regit_identifiers::convert::isin_to_cusip;
///
/// let isin = Isin::parse("US0378331005").unwrap();
/// assert_eq!(isin_to_cusip(&isin).unwrap().as_str(), "037833100");
/// ```
pub fn isin_to_cusip(isin: &Isin) -> Result<Cusip, ConversionError> {
    match isin.country_code() {
        "US" | "CA" => Ok(Cusip::parse(isin.nsin())?),
        _ => Err(ConversionError::UnsupportedCountry),
    }
}

/// Wraps a [`Cusip`] into an ISIN for the given country.
///
/// The CUSIP becomes the ISIN's nine-character NSIN verbatim; `country`
/// supplies the prefix and a fresh ISIN check digit is computed. Conventional
/// callers pass `"US"` or `"CA"`, but any recognised ISIN prefix is accepted —
/// the structural conversion is well-defined for all of them.
///
/// # Errors
///
/// - [`ConversionError::UnsupportedCountry`] if `country` is not a recognised
///   ISIN prefix.
/// - [`ConversionError::NotConvertible`] if `country` is not exactly two
///   characters.
/// - [`ConversionError::Validation`] if the assembled string is not a valid
///   ISIN.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Cusip;
/// use regit_identifiers::convert::cusip_to_isin;
///
/// let cusip = Cusip::parse("037833100").unwrap();
/// assert_eq!(cusip_to_isin(&cusip, "US").unwrap().as_str(), "US0378331005");
/// ```
pub fn cusip_to_isin(cusip: &Cusip, country: &str) -> Result<Isin, ConversionError> {
    build_isin(country, cusip.as_str())
}

/// Extracts the [`Sedol`] embedded in a United Kingdom or Ireland ISIN.
///
/// For a `GB` or `IE` ISIN the seven-character SEDOL sits right-aligned in the
/// nine-character NSIN, left-padded with the two literal characters `00`. The
/// leading `00` is stripped and the remaining seven characters are re-parsed
/// through [`Sedol::parse`], verifying the SEDOL's own check digit.
///
/// # Errors
///
/// - [`ConversionError::UnsupportedCountry`] if the ISIN's country prefix is
///   not `GB` or `IE`; only those jurisdictions embed a SEDOL.
/// - [`ConversionError::NotConvertible`] if the NSIN does not begin with the
///   literal `00` padding a SEDOL requires.
/// - [`ConversionError::Validation`] if the seven characters that remain are
///   not themselves a valid SEDOL.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Isin;
/// use regit_identifiers::convert::isin_to_sedol;
///
/// let isin = Isin::parse("GB0002634946").unwrap();
/// assert_eq!(isin_to_sedol(&isin).unwrap().as_str(), "0263494");
/// ```
pub fn isin_to_sedol(isin: &Isin) -> Result<Sedol, ConversionError> {
    match isin.country_code() {
        "GB" | "IE" => {}
        _ => return Err(ConversionError::UnsupportedCountry),
    }
    let nsin = isin.nsin();
    // The SEDOL occupies the trailing seven characters; the first two must be
    // the literal "00" padding.
    let rest = nsin
        .strip_prefix("00")
        .ok_or(ConversionError::NotConvertible {
            reason: "GB/IE NSIN must begin with 00 padding a SEDOL",
        })?;
    Ok(Sedol::parse(rest)?)
}

/// Wraps a [`Sedol`] into an ISIN for the given country.
///
/// The seven-character SEDOL is left-padded with the two literal characters
/// `00` to form the nine-character NSIN; `country` supplies the prefix and a
/// fresh ISIN check digit is computed. Conventional callers pass `"GB"` or
/// `"IE"`.
///
/// # Errors
///
/// - [`ConversionError::UnsupportedCountry`] if `country` is not a recognised
///   ISIN prefix.
/// - [`ConversionError::NotConvertible`] if `country` is not exactly two
///   characters.
/// - [`ConversionError::Validation`] if the assembled string is not a valid
///   ISIN.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Sedol;
/// use regit_identifiers::convert::sedol_to_isin;
///
/// let sedol = Sedol::parse("0263494").unwrap();
/// assert_eq!(sedol_to_isin(&sedol, "GB").unwrap().as_str(), "GB0002634946");
/// ```
pub fn sedol_to_isin(sedol: &Sedol, country: &str) -> Result<Isin, ConversionError> {
    // The NSIN is "00" followed by the seven-character SEDOL.
    let mut nsin = [b'0'; NSIN_LEN];
    if let Some(slot) = nsin.get_mut(2..NSIN_LEN) {
        slot.copy_from_slice(sedol.as_bytes());
    }
    let nsin_str = core::str::from_utf8(&nsin).unwrap_or("");
    build_isin(country, nsin_str)
}

/// Extracts the [`Valor`] embedded in a Switzerland or Liechtenstein ISIN.
///
/// For a `CH` or `LI` ISIN the VALOR is left-padded with zeros to fill the
/// nine-character NSIN. The leading zeros are stripped — at least one digit is
/// always kept, so a NSIN of all zeros yields the VALOR `0` — and the result
/// is re-parsed through [`Valor::parse`].
///
/// # Errors
///
/// - [`ConversionError::UnsupportedCountry`] if the ISIN's country prefix is
///   not `CH` or `LI`; only those jurisdictions embed a VALOR.
/// - [`ConversionError::Validation`] if the stripped digit string is not
///   itself a valid VALOR — for instance because the NSIN contained a letter.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Isin;
/// use regit_identifiers::convert::isin_to_valor;
///
/// let isin = Isin::parse("CH0012138530").unwrap();
/// assert_eq!(isin_to_valor(&isin).unwrap().as_str(), "1213853");
/// ```
pub fn isin_to_valor(isin: &Isin) -> Result<Valor, ConversionError> {
    match isin.country_code() {
        "CH" | "LI" => {}
        _ => return Err(ConversionError::UnsupportedCountry),
    }
    let nsin = isin.nsin();
    // Strip leading zeros, keeping at least the final character so an
    // all-zero NSIN yields the VALOR "0" rather than an empty string.
    let trimmed = nsin.trim_start_matches('0');
    let valor = if trimmed.is_empty() {
        // The NSIN is all zeros; the VALOR is a single zero digit.
        "0"
    } else {
        trimmed
    };
    Ok(Valor::parse(valor)?)
}

/// Wraps a [`Valor`] into an ISIN for the given country.
///
/// The VALOR's one-to-nine digits are left-zero-padded to the nine-character
/// NSIN; `country` supplies the prefix and a fresh ISIN check digit is
/// computed. Conventional callers pass `"CH"` or `"LI"`.
///
/// # Errors
///
/// - [`ConversionError::UnsupportedCountry`] if `country` is not a recognised
///   ISIN prefix.
/// - [`ConversionError::NotConvertible`] if `country` is not exactly two
///   characters.
/// - [`ConversionError::Validation`] if the assembled string is not a valid
///   ISIN.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Valor;
/// use regit_identifiers::convert::valor_to_isin;
///
/// let valor = Valor::parse("1213853").unwrap();
/// assert_eq!(valor_to_isin(&valor, "CH").unwrap().as_str(), "CH0012138530");
/// ```
pub fn valor_to_isin(valor: &Valor, country: &str) -> Result<Isin, ConversionError> {
    // `build_isin` left-zero-pads the VALOR's digits into the NSIN field.
    build_isin(country, valor.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ValidationError;

    // ─── build_isin ──────────────────────────────────────────────────────

    #[test]
    fn build_isin_apple() {
        // The CUSIP wrapped with a fresh ISIN check digit reproduces the
        // real Apple ISIN exactly.
        let isin = build_isin("US", "037833100").unwrap();
        assert_eq!(isin.as_str(), "US0378331005");
    }

    #[test]
    fn build_isin_agrees_with_isin_parse() {
        // `build_isin` must produce exactly what `Isin::parse` accepts.
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
    fn build_isin_left_pads_short_nsin() {
        // A national number shorter than nine characters is left-zero-padded.
        let isin = build_isin("CH", "1213853").unwrap();
        assert_eq!(isin.as_str(), "CH0012138530");
        assert_eq!(isin.nsin(), "001213853");
    }

    #[test]
    fn build_isin_accepts_single_character_nsin() {
        // A one-character national number pads to nine zeros-then-digit.
        let isin = build_isin("US", "1").unwrap();
        assert_eq!(isin.nsin(), "000000001");
    }

    #[test]
    fn build_isin_rejects_unknown_country() {
        assert_eq!(
            build_isin("ZZ", "037833100"),
            Err(ConversionError::UnsupportedCountry)
        );
    }

    #[test]
    fn build_isin_rejects_wrong_country_length() {
        assert!(matches!(
            build_isin("USA", "037833100"),
            Err(ConversionError::NotConvertible { .. })
        ));
        assert!(matches!(
            build_isin("U", "037833100"),
            Err(ConversionError::NotConvertible { .. })
        ));
    }

    #[test]
    fn build_isin_rejects_empty_and_overlong_nsin() {
        assert!(matches!(
            build_isin("US", ""),
            Err(ConversionError::NotConvertible { .. })
        ));
        assert!(matches!(
            build_isin("US", "0123456789"),
            Err(ConversionError::NotConvertible { .. })
        ));
    }

    #[test]
    fn build_isin_rejects_bad_nsin_character() {
        // A lower-case or otherwise illegal NSIN character surfaces as a
        // validation error from the check-digit step.
        assert!(matches!(
            build_isin("US", "03783310a"),
            Err(ConversionError::Validation(_))
        ));
    }

    #[test]
    fn build_isin_rejects_non_ascii() {
        assert!(matches!(
            build_isin("US", "0378331é"),
            Err(ConversionError::NotConvertible { .. })
        ));
        assert!(matches!(
            build_isin("ÉS", "037833100"),
            Err(ConversionError::NotConvertible { .. })
        ));
    }

    // ─── ISIN ↔ CUSIP ────────────────────────────────────────────────────

    #[test]
    fn isin_to_cusip_apple() {
        let isin = Isin::parse("US0378331005").unwrap();
        assert_eq!(isin_to_cusip(&isin).unwrap().as_str(), "037833100");
    }

    #[test]
    fn isin_to_cusip_accepts_canada() {
        // A CA ISIN whose NSIN is a valid CUSIP converts cleanly.
        let isin = cusip_to_isin(&Cusip::parse("037833100").unwrap(), "CA").unwrap();
        assert_eq!(isin.country_code(), "CA");
        assert_eq!(isin_to_cusip(&isin).unwrap().as_str(), "037833100");
    }

    #[test]
    fn isin_to_cusip_rejects_non_us_ca() {
        // A GB ISIN has no CUSIP, even though its NSIN is nine characters.
        let isin = Isin::parse("GB0002634946").unwrap();
        assert_eq!(
            isin_to_cusip(&isin),
            Err(ConversionError::UnsupportedCountry)
        );
        let de = Isin::parse("DE000BAY0017").unwrap();
        assert_eq!(isin_to_cusip(&de), Err(ConversionError::UnsupportedCountry));
    }

    #[test]
    fn cusip_to_isin_apple() {
        let cusip = Cusip::parse("037833100").unwrap();
        assert_eq!(
            cusip_to_isin(&cusip, "US").unwrap().as_str(),
            "US0378331005"
        );
    }

    #[test]
    fn cusip_to_isin_rejects_unknown_country() {
        let cusip = Cusip::parse("037833100").unwrap();
        assert_eq!(
            cusip_to_isin(&cusip, "ZZ"),
            Err(ConversionError::UnsupportedCountry)
        );
    }

    #[test]
    fn cusip_isin_round_trip() {
        // isin_to_cusip then cusip_to_isin recovers the original ISIN.
        for &s in &["US0378331005", "US5949181045"] {
            let isin = Isin::parse(s).unwrap();
            let cusip = isin_to_cusip(&isin).unwrap();
            let back = cusip_to_isin(&cusip, isin.country_code()).unwrap();
            assert_eq!(back, isin);
        }
        // And the reverse round-trip: CUSIP -> ISIN -> CUSIP.
        for &c in &["037833100", "594918104", "38259P508"] {
            let cusip = Cusip::parse(c).unwrap();
            let isin = cusip_to_isin(&cusip, "US").unwrap();
            assert_eq!(isin_to_cusip(&isin).unwrap(), cusip);
        }
    }

    // ─── ISIN ↔ SEDOL ────────────────────────────────────────────────────

    #[test]
    fn isin_to_sedol_bae() {
        let isin = Isin::parse("GB0002634946").unwrap();
        assert_eq!(isin_to_sedol(&isin).unwrap().as_str(), "0263494");
    }

    #[test]
    fn isin_to_sedol_accepts_ireland() {
        let isin = sedol_to_isin(&Sedol::parse("0263494").unwrap(), "IE").unwrap();
        assert_eq!(isin.country_code(), "IE");
        assert_eq!(isin_to_sedol(&isin).unwrap().as_str(), "0263494");
    }

    #[test]
    fn isin_to_sedol_rejects_non_gb_ie() {
        let isin = Isin::parse("US0378331005").unwrap();
        assert_eq!(
            isin_to_sedol(&isin),
            Err(ConversionError::UnsupportedCountry)
        );
    }

    #[test]
    fn isin_to_sedol_rejects_missing_00_padding() {
        // A GB ISIN whose NSIN does not begin with "00" cannot embed a SEDOL.
        // Build a valid GB ISIN with a non-"00" NSIN prefix.
        let isin = build_isin("GB", "123456789").unwrap();
        assert!(matches!(
            isin_to_sedol(&isin),
            Err(ConversionError::NotConvertible { .. })
        ));
    }

    #[test]
    fn isin_to_sedol_rejects_invalid_sedol_body() {
        // A GB ISIN with "00" padding but a body that is not a valid SEDOL
        // (a vowel is forbidden in a SEDOL) surfaces a validation error.
        let isin = build_isin("GB", "00B0WNLA7").unwrap();
        assert!(matches!(
            isin_to_sedol(&isin),
            Err(ConversionError::Validation(_))
        ));
    }

    #[test]
    fn sedol_to_isin_bae() {
        let sedol = Sedol::parse("0263494").unwrap();
        assert_eq!(
            sedol_to_isin(&sedol, "GB").unwrap().as_str(),
            "GB0002634946"
        );
    }

    #[test]
    fn sedol_to_isin_rejects_unknown_country() {
        let sedol = Sedol::parse("0263494").unwrap();
        assert_eq!(
            sedol_to_isin(&sedol, "ZZ"),
            Err(ConversionError::UnsupportedCountry)
        );
    }

    #[test]
    fn sedol_isin_round_trip() {
        // isin_to_sedol then sedol_to_isin recovers the original ISIN.
        let isin = Isin::parse("GB0002634946").unwrap();
        let sedol = isin_to_sedol(&isin).unwrap();
        let back = sedol_to_isin(&sedol, isin.country_code()).unwrap();
        assert_eq!(back, isin);

        // And SEDOL -> ISIN -> SEDOL for several SEDOLs.
        for &s in &["0263494", "0540528", "B0WNLY7"] {
            let sedol = Sedol::parse(s).unwrap();
            let isin = sedol_to_isin(&sedol, "GB").unwrap();
            assert_eq!(isin_to_sedol(&isin).unwrap(), sedol);
        }
    }

    // ─── ISIN ↔ VALOR ────────────────────────────────────────────────────

    #[test]
    fn isin_to_valor_strips_leading_zeros() {
        let isin = Isin::parse("CH0012138530").unwrap();
        assert_eq!(isin_to_valor(&isin).unwrap().as_str(), "1213853");
    }

    #[test]
    fn isin_to_valor_accepts_liechtenstein() {
        let isin = valor_to_isin(&Valor::parse("1213853").unwrap(), "LI").unwrap();
        assert_eq!(isin.country_code(), "LI");
        assert_eq!(isin_to_valor(&isin).unwrap().as_str(), "1213853");
    }

    #[test]
    fn isin_to_valor_all_zero_nsin_yields_zero() {
        // An all-zero NSIN strips to the single VALOR digit "0", not "".
        let isin = build_isin("CH", "000000000").unwrap();
        let valor = isin_to_valor(&isin).unwrap();
        assert_eq!(valor.as_str(), "0");
        assert_eq!(valor.as_u64(), 0);
    }

    #[test]
    fn isin_to_valor_rejects_non_ch_li() {
        let isin = Isin::parse("US0378331005").unwrap();
        assert_eq!(
            isin_to_valor(&isin),
            Err(ConversionError::UnsupportedCountry)
        );
    }

    #[test]
    fn isin_to_valor_rejects_non_numeric_nsin() {
        // A CH ISIN whose NSIN contains a letter cannot embed a VALOR.
        let isin = build_isin("CH", "00ABC1234").unwrap();
        assert!(matches!(
            isin_to_valor(&isin),
            Err(ConversionError::Validation(_))
        ));
    }

    #[test]
    fn valor_to_isin_pads_to_nine_digits() {
        let valor = Valor::parse("1213853").unwrap();
        let isin = valor_to_isin(&valor, "CH").unwrap();
        assert_eq!(isin.as_str(), "CH0012138530");
        assert_eq!(isin.nsin(), "001213853");
    }

    #[test]
    fn valor_to_isin_rejects_unknown_country() {
        let valor = Valor::parse("1213853").unwrap();
        assert_eq!(
            valor_to_isin(&valor, "ZZ"),
            Err(ConversionError::UnsupportedCountry)
        );
    }

    #[test]
    fn valor_isin_round_trip() {
        // isin_to_valor then valor_to_isin recovers the original ISIN.
        let isin = Isin::parse("CH0012138530").unwrap();
        let valor = isin_to_valor(&isin).unwrap();
        let back = valor_to_isin(&valor, isin.country_code()).unwrap();
        assert_eq!(back, isin);

        // And VALOR -> ISIN -> VALOR for several VALORs, including a
        // nine-digit one that exactly fills the NSIN.
        for &v in &["1213853", "908440", "24476758", "123456789", "7"] {
            let valor = Valor::parse(v).unwrap();
            let isin = valor_to_isin(&valor, "CH").unwrap();
            assert_eq!(isin_to_valor(&isin).unwrap(), valor);
        }
    }

    // ─── Cross-cutting ───────────────────────────────────────────────────

    #[test]
    fn check_digit_schemes_are_independent() {
        // The CUSIP check digit ('0' for Apple) and the ISIN check digit
        // ('5' for Apple) are computed by unrelated algorithms; the
        // conversion recomputes the ISIN digit rather than reusing the
        // CUSIP one.
        let cusip = Cusip::parse("037833100").unwrap();
        assert_eq!(cusip.check_digit(), '0');
        let isin = cusip_to_isin(&cusip, "US").unwrap();
        assert_eq!(isin.check_digit(), '5');
    }

    #[test]
    fn conversion_error_carries_validation_source() {
        // A failed inner validation is surfaced as ConversionError::Validation
        // and never silently turned into a wrong answer.
        let isin = build_isin("GB", "000000000").unwrap();
        // NSIN "000000000" -> SEDOL body "0000000" has the wrong check digit
        // unless it happens to be valid; assert the error is typed, whatever
        // it is, rather than a wrong Sedol.
        match isin_to_sedol(&isin) {
            Ok(s) => {
                // If it parses, it must be a genuine valid SEDOL.
                assert!(Sedol::validate(s.as_str()).is_ok());
            }
            Err(ConversionError::Validation(ValidationError::BadCheckDigit { .. })) => {}
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn every_built_isin_reparses() {
        // Whatever the inputs, a successfully built ISIN always re-parses —
        // the conversion never emits a structurally invalid identifier.
        for &(country, nsin) in &[
            ("US", "037833100"),
            ("GB", "000263494"),
            ("CH", "001213853"),
            ("XS", "174878390"),
        ] {
            if let Ok(isin) = build_isin(country, nsin) {
                assert!(Isin::validate(isin.as_str()).is_ok());
            }
        }
    }
}
