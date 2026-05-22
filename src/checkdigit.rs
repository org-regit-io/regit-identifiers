// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Check-digit algorithms for securities identifiers.
//!
//! A check digit is a redundant character appended to an identifier so that
//! a single mistyped or transposed character is detected rather than
//! silently accepted. Five of the identifiers in this crate carry one, and
//! each computes it differently — the scan direction, the letter expansion,
//! and the treatment of two-digit products are all load-bearing and easy to
//! get subtly wrong.
//!
//! Each function takes the identifier **body** — the identifier without its
//! check digit(s) — validates that body's own length and character set
//! defensively, and returns the check digit(s) the governing standard
//! prescribes. A parser verifies a supplied check digit by recomputing it
//! with the matching function and comparing; it never trusts the digit it
//! was given.
//!
//! # The algorithms
//!
//! ```text
//! luhn_checksum   Luhn mod-10 over a pure-digit string. Right-to-left,
//!                 rightmost digit weight 2, alternating 2,1,2,...; a
//!                 weighted product of 10 or more is folded to its digit
//!                 sum (equivalently, p - 9).
//!
//! isin (ISO 6166) Each body character is first expanded — a digit stays a
//!                 digit; a letter becomes the two-digit number 10 + (c -
//!                 'A') — and a Luhn mod-10 is taken over the resulting
//!                 digit string. Parity is assigned AFTER expansion.
//!
//! cusip (X9.6)    Modulus-10 "double add double". Left-to-right, 1-indexed:
//!                 odd positions weight 1, even positions weight 2. Each
//!                 weighted product is folded to floor(p/10) + (p mod 10).
//!
//! sedol           Fixed weight vector [1,3,1,7,3,9] applied left-to-right.
//!                 Unlike the others, weighted products are NOT folded.
//!
//! lei (ISO 7064)  MOD 97-10: expand the body followed by the literal "00",
//!                 read as one integer M; the check digits are 98 - (M mod
//!                 97). The modulus is taken by a streaming recurrence — no
//!                 wide integer is ever formed.
//!
//! figi (X9.145)   Modulus-10 double add double, but right-to-left with the
//!                 RIGHTMOST character at weight 1 (not 2); every decimal
//!                 digit of each weighted product is summed.
//! ```
//!
//! Every example below is a real, well-known instrument whose check digit
//! was recomputed by hand.
//!
//! # References
//!
//! - ISO 6166 — International Securities Identification Number (ISIN).
//! - ISO/IEC 7064 — Check character systems (the MOD 97-10 system).
//! - ANSI X9.6 — CUSIP, CUSIP Global Services.
//! - ANSI X9.145 / Object Management Group — Financial Instrument Global
//!   Identifier (FIGI), the `OpenFIGI` specification.
//! - London Stock Exchange — SEDOL Masterfile service description.

use crate::charset;
use crate::errors::ValidationError;

// ─── Shared primitives ───────────────────────────────────────────────────────

/// Maps a computed check value to its ASCII digit character. The value is
/// reduced modulo 10, so an input of `10` (the un-normalised result of
/// `10 - 0`) maps to `'0'`.
#[inline]
fn digit_char(value: u32) -> char {
    char::from(b'0' + u8::try_from(value % 10).unwrap_or(0))
}

/// Luhn contribution of a single digit `d` (`0..=9`). When `doubled` is
/// `true` the digit is weighted by 2 and a product of 10 or more is folded
/// to the sum of its two digits (equivalently `p - 9`, since `p` cannot
/// exceed 18); otherwise the digit contributes its own value.
#[inline]
fn luhn_contribution(d: u32, doubled: bool) -> u32 {
    let weighted = if doubled { d * 2 } else { d };
    if weighted > 9 { weighted - 9 } else { weighted }
}

/// `true` if `ch` is an upper-case ASCII vowel, which SEDOL and FIGI bodies
/// forbid.
#[inline]
fn is_vowel(ch: char) -> bool {
    matches!(ch, 'A' | 'E' | 'I' | 'O' | 'U')
}

/// Numeric value of a CUSIP body character: a digit is its own value, a
/// letter is `10 + (c - 'A')`, and the three special characters extend the
/// alphabet (`* = 36`, `@ = 37`, `# = 38`). The caller must already have
/// established that `b` is a legal CUSIP body byte.
#[inline]
fn cusip_value(b: u8) -> u32 {
    match b {
        b'*' => 36,
        b'@' => 37,
        b'#' => 38,
        _ => charset::alnum_value(b),
    }
}

// ─── Luhn mod-10 ─────────────────────────────────────────────────────────────

/// Computes the Luhn mod-10 checksum digit of a pure-digit string.
///
/// The string is scanned right-to-left; the rightmost digit carries weight
/// 2, and the weight then alternates 1, 2, 1, ... A weighted product of 10
/// or more is folded to the sum of its digits. The checksum digit is
/// `(10 - (sum mod 10)) mod 10` — the digit that, appended on the right,
/// makes the whole string pass a Luhn check.
///
/// # Errors
///
/// - [`ValidationError::Empty`] if `digits` is empty.
/// - [`ValidationError::InvalidCharacter`] if any character is not an ASCII
///   decimal digit.
///
/// # Examples
///
/// ```
/// use regit_identifiers::checkdigit::luhn_checksum;
///
/// // The canonical Luhn example: "7992739871" has checksum digit 3.
/// assert_eq!(luhn_checksum("7992739871").unwrap(), 3);
/// ```
pub fn luhn_checksum(digits: &str) -> Result<u8, ValidationError> {
    if digits.is_empty() {
        return Err(ValidationError::Empty);
    }
    for (i, ch) in digits.chars().enumerate() {
        if !ch.is_ascii_digit() {
            return Err(ValidationError::InvalidCharacter {
                position: i + 1,
                found: ch,
            });
        }
    }
    // Right-to-left: the rightmost digit is doubled, then doubling alternates.
    let mut sum = 0u32;
    let mut doubled = true;
    for &b in digits.as_bytes().iter().rev() {
        sum += luhn_contribution(charset::digit_value(b), doubled);
        doubled = !doubled;
    }
    Ok(u8::try_from((10 - (sum % 10)) % 10).unwrap_or(0))
}

// ─── ISIN — ISO 6166 ─────────────────────────────────────────────────────────

/// Computes the ISIN check digit (ISO 6166) of an 11-character body.
///
/// The body is the country prefix plus the NSIN — the ISIN without its final
/// digit. Each character is expanded (a digit stays itself; a letter becomes
/// the two-digit number `10 + (c - 'A')`), and the Luhn mod-10 is taken over
/// the expanded digit string. Crucially, the alternating Luhn weights are
/// assigned over the *expanded* string, not the original characters.
///
/// # Errors
///
/// - [`ValidationError::WrongLength`] if the body is not exactly 11
///   characters.
/// - [`ValidationError::InvalidCharacter`] if any character is not an ASCII
///   digit or upper-case letter.
///
/// # Examples
///
/// ```
/// use regit_identifiers::checkdigit::isin_check_digit;
///
/// // Apple Inc., ISIN US0378331005 — body "US037833100", check digit 5.
/// assert_eq!(isin_check_digit("US037833100").unwrap(), '5');
/// ```
pub fn isin_check_digit(body: &str) -> Result<char, ValidationError> {
    const LEN: usize = 11;
    let found = body.chars().count();
    if found != LEN {
        return Err(ValidationError::WrongLength {
            expected: LEN,
            found,
        });
    }
    for (i, ch) in body.chars().enumerate() {
        if !(ch.is_ascii_digit() || ch.is_ascii_uppercase()) {
            return Err(ValidationError::InvalidCharacter {
                position: i + 1,
                found: ch,
            });
        }
    }
    // Every character is an ASCII alphanumeric, so the body is exactly 11
    // ASCII bytes. Expand right-to-left: a digit emits one expanded digit; a
    // letter emits the two digits of 10 + (c - 'A'), with the units digit
    // lying to the right of the tens digit in the expanded string.
    let mut sum = 0u32;
    let mut doubled = true;
    for &b in body.as_bytes().iter().rev() {
        let value = charset::alnum_value(b); // digit -> 0..=9, letter -> 10..=35
        if value < 10 {
            sum += luhn_contribution(value, doubled);
            doubled = !doubled;
        } else {
            sum += luhn_contribution(value % 10, doubled);
            doubled = !doubled;
            sum += luhn_contribution(value / 10, doubled);
            doubled = !doubled;
        }
    }
    Ok(digit_char(10 - (sum % 10)))
}

// ─── CUSIP — ANSI X9.6 ───────────────────────────────────────────────────────

/// Computes the CUSIP check digit (ANSI X9.6) of an 8-character body.
///
/// The algorithm is the "modulus 10 double add double": the body is scanned
/// left-to-right and, with 1-based positions, odd positions take weight 1
/// and even positions weight 2. Each weighted product is folded to
/// `floor(p / 10) + (p mod 10)`, the products are summed, and the check
/// digit is `(10 - (sum mod 10)) mod 10`. The body alphabet is the digits,
/// the upper-case letters, and the three special characters `*`, `@`, `#`.
/// The same algorithm computes a CINS check digit.
///
/// # Errors
///
/// - [`ValidationError::WrongLength`] if the body is not exactly 8
///   characters.
/// - [`ValidationError::InvalidCharacter`] if any character is not a digit,
///   an upper-case letter, or one of `*`, `@`, `#`.
///
/// # Examples
///
/// ```
/// use regit_identifiers::checkdigit::cusip_check_digit;
///
/// // Apple Inc., CUSIP 037833100 — body "03783310", check digit 0.
/// assert_eq!(cusip_check_digit("03783310").unwrap(), '0');
/// ```
pub fn cusip_check_digit(body: &str) -> Result<char, ValidationError> {
    const LEN: usize = 8;
    let found = body.chars().count();
    if found != LEN {
        return Err(ValidationError::WrongLength {
            expected: LEN,
            found,
        });
    }
    for (i, ch) in body.chars().enumerate() {
        let legal = ch.is_ascii_digit() || ch.is_ascii_uppercase() || matches!(ch, '*' | '@' | '#');
        if !legal {
            return Err(ValidationError::InvalidCharacter {
                position: i + 1,
                found: ch,
            });
        }
    }
    let mut sum = 0u32;
    for (i, &b) in body.as_bytes().iter().enumerate() {
        // 1-based position i + 1: odd -> weight 1, even -> weight 2.
        let weight = if i % 2 == 0 { 1 } else { 2 };
        let product = cusip_value(b) * weight;
        sum += product / 10 + product % 10;
    }
    Ok(digit_char(10 - (sum % 10)))
}

// ─── SEDOL — London Stock Exchange ───────────────────────────────────────────

/// Computes the SEDOL check digit of a 6-character body.
///
/// The six characters are weighted left-to-right by the fixed vector
/// `[1, 3, 1, 7, 3, 9]` and the weighted values are summed; the check digit
/// is `(10 - (sum mod 10)) mod 10`. Unlike the ISIN, CUSIP, and FIGI
/// algorithms, SEDOL does **not** fold a two-digit weighted product to its
/// digit sum. The body alphabet is the digits and the consonants — a SEDOL
/// never contains a vowel.
///
/// # Errors
///
/// - [`ValidationError::WrongLength`] if the body is not exactly 6
///   characters.
/// - [`ValidationError::InvalidCharacter`] if any character is not a digit
///   or an upper-case consonant (a vowel is rejected here).
///
/// # Examples
///
/// ```
/// use regit_identifiers::checkdigit::sedol_check_digit;
///
/// // BAE Systems, SEDOL 0263494 — body "026349", check digit 4.
/// assert_eq!(sedol_check_digit("026349").unwrap(), '4');
/// ```
pub fn sedol_check_digit(body: &str) -> Result<char, ValidationError> {
    const LEN: usize = 6;
    const WEIGHTS: [u32; LEN] = [1, 3, 1, 7, 3, 9];
    let found = body.chars().count();
    if found != LEN {
        return Err(ValidationError::WrongLength {
            expected: LEN,
            found,
        });
    }
    for (i, ch) in body.chars().enumerate() {
        let legal = ch.is_ascii_digit() || (ch.is_ascii_uppercase() && !is_vowel(ch));
        if !legal {
            return Err(ValidationError::InvalidCharacter {
                position: i + 1,
                found: ch,
            });
        }
    }
    let mut sum = 0u32;
    for (&b, &weight) in body.as_bytes().iter().zip(WEIGHTS.iter()) {
        sum += charset::alnum_value(b) * weight;
    }
    Ok(digit_char(10 - (sum % 10)))
}

// ─── LEI — ISO 17442 / ISO 7064 MOD 97-10 ────────────────────────────────────

/// Computes the two LEI check digits (ISO 7064 MOD 97-10) of an
/// 18-character body.
///
/// The body and the literal string `"00"` are expanded (a digit stays
/// itself; a letter becomes `10 + (c - 'A')`) into one large integer `M`;
/// the check digits are `98 - (M mod 97)`, written as two digits. The
/// modulus is computed by the streaming recurrence `acc = (acc * 10 + d) mod
/// 97` for a digit and `acc = (acc * 100 + v) mod 97` for an expanded
/// letter, so the 38-or-so-digit integer is never actually formed.
///
/// # Errors
///
/// - [`ValidationError::WrongLength`] if the body is not exactly 18
///   characters.
/// - [`ValidationError::InvalidCharacter`] if any character is not an ASCII
///   digit or upper-case letter.
///
/// # Examples
///
/// ```
/// use regit_identifiers::checkdigit::lei_check_digits;
///
/// // Bloomberg Finance L.P., LEI 5493001KJTIIGC8Y1R12 —
/// // body "5493001KJTIIGC8Y1R", check digits "12".
/// assert_eq!(lei_check_digits("5493001KJTIIGC8Y1R").unwrap(), ['1', '2']);
/// ```
pub fn lei_check_digits(body: &str) -> Result<[char; 2], ValidationError> {
    const LEN: usize = 18;
    let found = body.chars().count();
    if found != LEN {
        return Err(ValidationError::WrongLength {
            expected: LEN,
            found,
        });
    }
    for (i, ch) in body.chars().enumerate() {
        if !(ch.is_ascii_digit() || ch.is_ascii_uppercase()) {
            return Err(ValidationError::InvalidCharacter {
                position: i + 1,
                found: ch,
            });
        }
    }
    // Streaming ISO 7064 MOD 97-10. A digit contributes one decimal place,
    // an expanded letter (10..=35) contributes two. The accumulator is a
    // residue mod 97, so it is at most 96 and the largest intermediate value
    // is 96 * 100 + 35 = 9635 — far within `u32`.
    let mut acc = 0u32;
    for &b in body.as_bytes() {
        let value = charset::alnum_value(b);
        if value < 10 {
            acc = (acc * 10 + value) % 97;
        } else {
            acc = (acc * 100 + value) % 97;
        }
    }
    // Append the two check positions as the literal "00".
    acc = (acc * 100) % 97;
    let check = 98 - acc; // in 2..=98
    Ok([digit_char(check / 10), digit_char(check % 10)])
}

// ─── FIGI — ANSI X9.145 ──────────────────────────────────────────────────────

/// Computes the FIGI check digit (ANSI X9.145) of an 11-character body.
///
/// The algorithm is a modulus-10 double add double scanned right-to-left,
/// but — unlike a plain Luhn — the rightmost character carries weight 1, not
/// 2; the weight then alternates 2, 1, 2, ... Every decimal digit of each
/// weighted product is added to the running sum, and the check digit is
/// `(10 - (sum mod 10)) mod 10`. The body alphabet is the digits and the
/// consonants — a FIGI never contains a vowel.
///
/// # Errors
///
/// - [`ValidationError::WrongLength`] if the body is not exactly 11
///   characters.
/// - [`ValidationError::InvalidCharacter`] if any character is not a digit
///   or an upper-case consonant (a vowel is rejected here).
///
/// # Examples
///
/// ```
/// use regit_identifiers::checkdigit::figi_check_digit;
///
/// // IBM, FIGI BBG000BLNNH6 — body "BBG000BLNNH", check digit 6.
/// assert_eq!(figi_check_digit("BBG000BLNNH").unwrap(), '6');
/// ```
pub fn figi_check_digit(body: &str) -> Result<char, ValidationError> {
    const LEN: usize = 11;
    let found = body.chars().count();
    if found != LEN {
        return Err(ValidationError::WrongLength {
            expected: LEN,
            found,
        });
    }
    for (i, ch) in body.chars().enumerate() {
        let legal = ch.is_ascii_digit() || (ch.is_ascii_uppercase() && !is_vowel(ch));
        if !legal {
            return Err(ValidationError::InvalidCharacter {
                position: i + 1,
                found: ch,
            });
        }
    }
    // Right-to-left: the rightmost character (position 0) has weight 1, then
    // the weight alternates 2, 1, 2, ... Every decimal digit of the weighted
    // product is summed (the product cannot exceed 35 * 2 = 70).
    let mut sum = 0u32;
    let mut doubled = false; // the rightmost character has weight 1
    for &b in body.as_bytes().iter().rev() {
        let weight = if doubled { 2 } else { 1 };
        let product = charset::alnum_value(b) * weight;
        sum += product / 10 + product % 10;
        doubled = !doubled;
    }
    Ok(digit_char(10 - (sum % 10)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Luhn ────────────────────────────────────────────────────────────

    #[test]
    fn luhn_canonical_example() {
        assert_eq!(luhn_checksum("7992739871").unwrap(), 3);
    }

    #[test]
    fn luhn_rejects_empty_and_non_digit() {
        assert_eq!(luhn_checksum(""), Err(ValidationError::Empty));
        assert_eq!(
            luhn_checksum("12A4"),
            Err(ValidationError::InvalidCharacter {
                position: 3,
                found: 'A',
            })
        );
    }

    // ─── ISIN ────────────────────────────────────────────────────────────

    #[test]
    fn isin_worked_example_apple() {
        // US0378331005 — body "US037833100", check digit 5 (hand-computed).
        assert_eq!(isin_check_digit("US037833100").unwrap(), '5');
    }

    #[test]
    fn isin_golden_vectors() {
        // Real instruments; the check digit is each ISIN's final character.
        assert_eq!(isin_check_digit("US594918104").unwrap(), '5'); // US5949181045, Microsoft
        assert_eq!(isin_check_digit("GB000263494").unwrap(), '6'); // GB0002634946, BAE Systems
        assert_eq!(isin_check_digit("DE000BAY001").unwrap(), '7'); // DE000BAY0017, Bayer
    }

    #[test]
    fn isin_rejects_wrong_length() {
        assert_eq!(
            isin_check_digit("US03783310"),
            Err(ValidationError::WrongLength {
                expected: 11,
                found: 10,
            })
        );
    }

    #[test]
    fn isin_rejects_bad_character() {
        assert_eq!(
            isin_check_digit("US0378331/0"),
            Err(ValidationError::InvalidCharacter {
                position: 10,
                found: '/',
            })
        );
        // Lower case is rejected — identifiers are upper-case only.
        assert!(matches!(
            isin_check_digit("us037833100"),
            Err(ValidationError::InvalidCharacter { .. })
        ));
    }

    // ─── CUSIP ───────────────────────────────────────────────────────────

    #[test]
    fn cusip_worked_example_apple() {
        // 037833100 — body "03783310", check digit 0 (hand-computed).
        assert_eq!(cusip_check_digit("03783310").unwrap(), '0');
    }

    #[test]
    fn cusip_golden_vectors() {
        // Real instruments; the check digit is each CUSIP's final character.
        assert_eq!(cusip_check_digit("59491810").unwrap(), '4'); // 594918104, Microsoft
        assert_eq!(cusip_check_digit("38259P50").unwrap(), '8'); // 38259P508, Alphabet
    }

    #[test]
    fn cusip_rejects_wrong_length_and_char() {
        assert_eq!(
            cusip_check_digit("0378331"),
            Err(ValidationError::WrongLength {
                expected: 8,
                found: 7,
            })
        );
        assert!(matches!(
            cusip_check_digit("0378331."),
            Err(ValidationError::InvalidCharacter { .. })
        ));
    }

    // ─── SEDOL ───────────────────────────────────────────────────────────

    #[test]
    fn sedol_worked_example_bae() {
        // 0263494 — body "026349", check digit 4 (hand-computed).
        assert_eq!(sedol_check_digit("026349").unwrap(), '4');
    }

    #[test]
    fn sedol_golden_vectors() {
        assert_eq!(sedol_check_digit("B0WNLY").unwrap(), '7'); // B0WNLY7
        assert_eq!(sedol_check_digit("054052").unwrap(), '8'); // 0540528
    }

    #[test]
    fn sedol_rejects_vowel() {
        // A vowel can never appear in a SEDOL body.
        assert_eq!(
            sedol_check_digit("B0WNLA"),
            Err(ValidationError::InvalidCharacter {
                position: 6,
                found: 'A',
            })
        );
    }

    #[test]
    fn sedol_rejects_wrong_length() {
        assert_eq!(
            sedol_check_digit("02634"),
            Err(ValidationError::WrongLength {
                expected: 6,
                found: 5,
            })
        );
    }

    // ─── LEI ─────────────────────────────────────────────────────────────

    #[test]
    fn lei_worked_example_bloomberg() {
        // 5493001KJTIIGC8Y1R12 — body "5493001KJTIIGC8Y1R", check "12".
        assert_eq!(lei_check_digits("5493001KJTIIGC8Y1R").unwrap(), ['1', '2']);
    }

    #[test]
    fn lei_golden_vectors() {
        // 549300DTUYXVMJXZNY75 — a second real LEI beyond the worked example.
        assert_eq!(lei_check_digits("549300DTUYXVMJXZNY").unwrap(), ['7', '5']);
    }

    #[test]
    fn lei_rejects_wrong_length_and_char() {
        assert_eq!(
            lei_check_digits("5493001KJTIIGC8Y1"),
            Err(ValidationError::WrongLength {
                expected: 18,
                found: 17,
            })
        );
        assert!(matches!(
            lei_check_digits("5493001KJTIIGC8Y1-"),
            Err(ValidationError::InvalidCharacter { .. })
        ));
    }

    // ─── FIGI ────────────────────────────────────────────────────────────

    #[test]
    fn figi_worked_example_ibm() {
        // BBG000BLNNH6 — body "BBG000BLNNH", check digit 6 (hand-computed).
        assert_eq!(figi_check_digit("BBG000BLNNH").unwrap(), '6');
    }

    #[test]
    fn figi_golden_vectors() {
        assert_eq!(figi_check_digit("BBG000B9XRY").unwrap(), '4'); // BBG000B9XRY4
        assert_eq!(figi_check_digit("BBG000BVPV8").unwrap(), '4'); // BBG000BVPV84
        assert_eq!(figi_check_digit("BBG0013T5HY").unwrap(), '0'); // BBG0013T5HY0
    }

    #[test]
    fn figi_rejects_vowel() {
        // FIGI bodies forbid vowels.
        assert!(matches!(
            figi_check_digit("BBG00OBLNNH"),
            Err(ValidationError::InvalidCharacter { .. })
        ));
    }

    #[test]
    fn figi_rejects_wrong_length() {
        assert_eq!(
            figi_check_digit("BBG000BLNN"),
            Err(ValidationError::WrongLength {
                expected: 11,
                found: 10,
            })
        );
    }

    // ─── Cross-cutting ───────────────────────────────────────────────────

    #[test]
    fn every_check_digit_is_an_ascii_digit() {
        assert!(isin_check_digit("US037833100").unwrap().is_ascii_digit());
        assert!(cusip_check_digit("03783310").unwrap().is_ascii_digit());
        assert!(sedol_check_digit("026349").unwrap().is_ascii_digit());
        assert!(figi_check_digit("BBG000BLNNH").unwrap().is_ascii_digit());
        let lei = lei_check_digits("5493001KJTIIGC8Y1R").unwrap();
        assert!(lei[0].is_ascii_digit() && lei[1].is_ascii_digit());
    }

    #[test]
    fn non_ascii_input_is_rejected_not_panicked() {
        // A multi-byte character must be rejected cleanly, never panic.
        assert!(isin_check_digit("US03783310é").is_err());
        assert!(cusip_check_digit("0378331é").is_err());
        assert!(lei_check_digits("5493001KJTIIGC8Y1é").is_err());
    }
}
