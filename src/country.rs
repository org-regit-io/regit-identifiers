// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! ISO 3166-1 alpha-2 country codes and ISIN country-prefix rules.
//!
//! An ISIN, a BIC, and several national identifiers embed a two-letter
//! country code. This module answers two questions about such a code:
//! whether it is a current ISO 3166-1 alpha-2 code, and — separately —
//! whether it is admissible as the leading prefix of an ISIN.
//!
//! The two questions differ. An ISIN prefix is *usually* an ISO 3166-1
//! country code, but the ISIN standard also admits a small set of
//! **substitute prefixes** for securities with no single national home —
//! `XS` for internationally-cleared securities, `EU` for European Union
//! institutions, and a few others. A validator that accepts only ISO
//! country codes would wrongly reject every Eurobond ISIN; one that treats
//! the substitutes as ISO countries would wrongly accept them elsewhere.
//! The two predicates [`is_iso_country`] and [`is_isin_prefix`] keep the
//! distinction exact.
//!
//! The embedded table holds all 249 officially assigned ISO 3166-1 alpha-2
//! codes, sorted by code for binary search. It reflects the ISO 3166-1
//! standard as maintained by the ISO 3166 Maintenance Agency; the snapshot
//! is current as of 2026-05.
//!
//! # References
//!
//! - ISO 3166-1, *Codes for the representation of names of countries and
//!   their subdivisions — Part 1: Country code*.
//! - ISO 6166 (ISIN) — the standard whose prefix field admits the
//!   substitute codes enumerated in [`is_isin_prefix`].

/// ISIN substitute prefixes: two-letter codes admissible as an ISIN prefix
/// that are *not* ISO 3166-1 country codes.
///
/// They name issuers and instruments with no single national home, such as
/// internationally-cleared securities (`XS`) and European Union institutions
/// (`EU`). The set is fixed by the ISIN standard and its registration
/// authority; it is sorted for binary search.
const ISIN_SUBSTITUTE_PREFIXES: &[&str] =
    &["EU", "QS", "QT", "XA", "XB", "XC", "XD", "XF", "XK", "XS"];

/// Returns `true` if `code` is a current ISO 3166-1 alpha-2 country code.
///
/// The match is exact and case-sensitive: `code` must be the two upper-case
/// letters of the standard. Any other input — lower case, wrong length,
/// a substitute prefix — returns `false`.
///
/// # Examples
///
/// ```
/// use regit_identifiers::country::is_iso_country;
///
/// assert!(is_iso_country("US"));
/// assert!(is_iso_country("DE"));
/// assert!(!is_iso_country("us"));   // case-sensitive
/// assert!(!is_iso_country("ZZ"));   // not assigned
/// assert!(!is_iso_country("XS"));   // an ISIN substitute, not a country
/// ```
#[must_use]
pub fn is_iso_country(code: &str) -> bool {
    COUNTRIES.binary_search_by(|&(c, _)| c.cmp(code)).is_ok()
}

/// Returns the English short name of an ISO 3166-1 alpha-2 country code,
/// or `None` if `code` is not an assigned code.
///
/// # Examples
///
/// ```
/// use regit_identifiers::country::country_name;
///
/// assert_eq!(country_name("FR"), Some("France"));
/// assert_eq!(country_name("ZZ"), None);
/// ```
#[must_use]
pub fn country_name(code: &str) -> Option<&'static str> {
    COUNTRIES
        .binary_search_by(|&(c, _)| c.cmp(code))
        .ok()
        .map(|index| COUNTRIES[index].1)
}

/// Returns `true` if `code` is admissible as the two-letter prefix of an
/// ISIN.
///
/// This is `true` for every ISO 3166-1 country code ([`is_iso_country`]) and,
/// in addition, for each ISIN substitute prefix (`EU`, `QS`, `QT`, `XA`,
/// `XB`, `XC`, `XD`, `XF`, `XK`, `XS`). Use this — not [`is_iso_country`] —
/// when validating an ISIN, or every internationally-cleared security would
/// be wrongly rejected.
///
/// # Examples
///
/// ```
/// use regit_identifiers::country::is_isin_prefix;
///
/// assert!(is_isin_prefix("US"));   // an ISO country
/// assert!(is_isin_prefix("XS"));   // internationally-cleared securities
/// assert!(is_isin_prefix("EU"));   // European Union institutions
/// assert!(!is_isin_prefix("ZZ"));  // neither
/// ```
#[must_use]
pub fn is_isin_prefix(code: &str) -> bool {
    is_iso_country(code) || ISIN_SUBSTITUTE_PREFIXES.binary_search(&code).is_ok()
}

/// Returns the English description of a valid ISIN country prefix, or
/// `None` if `code` is not admissible as an ISIN prefix.
///
/// For an ISO 3166-1 alpha-2 code this is the country's English short name
/// (equivalent to [`country_name`]). For an ISIN substitute prefix — a code
/// valid in an ISIN but not in ISO 3166-1 — this is a short description of
/// what the prefix names:
///
/// - `XS` — Internationally cleared securities (Euroclear / Clearstream)
/// - `EU` — European Union institutions
/// - `XK` — Kosovo
/// - `XA`, `XB`, `XC`, `XD`, `XF`, `QS`, `QT` — ISIN substitute prefix
///
/// # Examples
///
/// ```
/// use regit_identifiers::country::isin_prefix_name;
///
/// assert_eq!(isin_prefix_name("US"), Some("United States of America"));
/// assert_eq!(isin_prefix_name("XS"), Some("Internationally cleared securities"));
/// assert_eq!(isin_prefix_name("EU"), Some("European Union institutions"));
/// assert_eq!(isin_prefix_name("ZZ"), None);
/// ```
#[must_use]
pub fn isin_prefix_name(code: &str) -> Option<&'static str> {
    if let Some(name) = country_name(code) {
        return Some(name);
    }
    match code {
        "XS" => Some("Internationally cleared securities"),
        "EU" => Some("European Union institutions"),
        "XK" => Some("Kosovo"),
        "XA" | "XB" | "XC" | "XD" | "XF" | "QS" | "QT" => Some("ISIN substitute prefix"),
        _ => None,
    }
}

/// All 249 officially assigned ISO 3166-1 alpha-2 codes, as
/// `(code, english_short_name)`, sorted by code for binary search.
static COUNTRIES: &[(&str, &str)] = &[
    ("AD", "Andorra"),
    ("AE", "United Arab Emirates"),
    ("AF", "Afghanistan"),
    ("AG", "Antigua and Barbuda"),
    ("AI", "Anguilla"),
    ("AL", "Albania"),
    ("AM", "Armenia"),
    ("AO", "Angola"),
    ("AQ", "Antarctica"),
    ("AR", "Argentina"),
    ("AS", "American Samoa"),
    ("AT", "Austria"),
    ("AU", "Australia"),
    ("AW", "Aruba"),
    ("AX", "Åland Islands"),
    ("AZ", "Azerbaijan"),
    ("BA", "Bosnia and Herzegovina"),
    ("BB", "Barbados"),
    ("BD", "Bangladesh"),
    ("BE", "Belgium"),
    ("BF", "Burkina Faso"),
    ("BG", "Bulgaria"),
    ("BH", "Bahrain"),
    ("BI", "Burundi"),
    ("BJ", "Benin"),
    ("BL", "Saint Barthélemy"),
    ("BM", "Bermuda"),
    ("BN", "Brunei Darussalam"),
    ("BO", "Bolivia, Plurinational State of"),
    ("BQ", "Bonaire, Sint Eustatius and Saba"),
    ("BR", "Brazil"),
    ("BS", "Bahamas"),
    ("BT", "Bhutan"),
    ("BV", "Bouvet Island"),
    ("BW", "Botswana"),
    ("BY", "Belarus"),
    ("BZ", "Belize"),
    ("CA", "Canada"),
    ("CC", "Cocos (Keeling) Islands"),
    ("CD", "Congo, Democratic Republic of the"),
    ("CF", "Central African Republic"),
    ("CG", "Congo"),
    ("CH", "Switzerland"),
    ("CI", "Côte d'Ivoire"),
    ("CK", "Cook Islands"),
    ("CL", "Chile"),
    ("CM", "Cameroon"),
    ("CN", "China"),
    ("CO", "Colombia"),
    ("CR", "Costa Rica"),
    ("CU", "Cuba"),
    ("CV", "Cabo Verde"),
    ("CW", "Curaçao"),
    ("CX", "Christmas Island"),
    ("CY", "Cyprus"),
    ("CZ", "Czechia"),
    ("DE", "Germany"),
    ("DJ", "Djibouti"),
    ("DK", "Denmark"),
    ("DM", "Dominica"),
    ("DO", "Dominican Republic"),
    ("DZ", "Algeria"),
    ("EC", "Ecuador"),
    ("EE", "Estonia"),
    ("EG", "Egypt"),
    ("EH", "Western Sahara"),
    ("ER", "Eritrea"),
    ("ES", "Spain"),
    ("ET", "Ethiopia"),
    ("FI", "Finland"),
    ("FJ", "Fiji"),
    ("FK", "Falkland Islands (Malvinas)"),
    ("FM", "Micronesia, Federated States of"),
    ("FO", "Faroe Islands"),
    ("FR", "France"),
    ("GA", "Gabon"),
    ("GB", "United Kingdom of Great Britain and Northern Ireland"),
    ("GD", "Grenada"),
    ("GE", "Georgia"),
    ("GF", "French Guiana"),
    ("GG", "Guernsey"),
    ("GH", "Ghana"),
    ("GI", "Gibraltar"),
    ("GL", "Greenland"),
    ("GM", "Gambia"),
    ("GN", "Guinea"),
    ("GP", "Guadeloupe"),
    ("GQ", "Equatorial Guinea"),
    ("GR", "Greece"),
    ("GS", "South Georgia and the South Sandwich Islands"),
    ("GT", "Guatemala"),
    ("GU", "Guam"),
    ("GW", "Guinea-Bissau"),
    ("GY", "Guyana"),
    ("HK", "Hong Kong"),
    ("HM", "Heard Island and McDonald Islands"),
    ("HN", "Honduras"),
    ("HR", "Croatia"),
    ("HT", "Haiti"),
    ("HU", "Hungary"),
    ("ID", "Indonesia"),
    ("IE", "Ireland"),
    ("IL", "Israel"),
    ("IM", "Isle of Man"),
    ("IN", "India"),
    ("IO", "British Indian Ocean Territory"),
    ("IQ", "Iraq"),
    ("IR", "Iran, Islamic Republic of"),
    ("IS", "Iceland"),
    ("IT", "Italy"),
    ("JE", "Jersey"),
    ("JM", "Jamaica"),
    ("JO", "Jordan"),
    ("JP", "Japan"),
    ("KE", "Kenya"),
    ("KG", "Kyrgyzstan"),
    ("KH", "Cambodia"),
    ("KI", "Kiribati"),
    ("KM", "Comoros"),
    ("KN", "Saint Kitts and Nevis"),
    ("KP", "Korea, Democratic People's Republic of"),
    ("KR", "Korea, Republic of"),
    ("KW", "Kuwait"),
    ("KY", "Cayman Islands"),
    ("KZ", "Kazakhstan"),
    ("LA", "Lao People's Democratic Republic"),
    ("LB", "Lebanon"),
    ("LC", "Saint Lucia"),
    ("LI", "Liechtenstein"),
    ("LK", "Sri Lanka"),
    ("LR", "Liberia"),
    ("LS", "Lesotho"),
    ("LT", "Lithuania"),
    ("LU", "Luxembourg"),
    ("LV", "Latvia"),
    ("LY", "Libya"),
    ("MA", "Morocco"),
    ("MC", "Monaco"),
    ("MD", "Moldova, Republic of"),
    ("ME", "Montenegro"),
    ("MF", "Saint Martin (French part)"),
    ("MG", "Madagascar"),
    ("MH", "Marshall Islands"),
    ("MK", "North Macedonia"),
    ("ML", "Mali"),
    ("MM", "Myanmar"),
    ("MN", "Mongolia"),
    ("MO", "Macao"),
    ("MP", "Northern Mariana Islands"),
    ("MQ", "Martinique"),
    ("MR", "Mauritania"),
    ("MS", "Montserrat"),
    ("MT", "Malta"),
    ("MU", "Mauritius"),
    ("MV", "Maldives"),
    ("MW", "Malawi"),
    ("MX", "Mexico"),
    ("MY", "Malaysia"),
    ("MZ", "Mozambique"),
    ("NA", "Namibia"),
    ("NC", "New Caledonia"),
    ("NE", "Niger"),
    ("NF", "Norfolk Island"),
    ("NG", "Nigeria"),
    ("NI", "Nicaragua"),
    ("NL", "Netherlands, Kingdom of the"),
    ("NO", "Norway"),
    ("NP", "Nepal"),
    ("NR", "Nauru"),
    ("NU", "Niue"),
    ("NZ", "New Zealand"),
    ("OM", "Oman"),
    ("PA", "Panama"),
    ("PE", "Peru"),
    ("PF", "French Polynesia"),
    ("PG", "Papua New Guinea"),
    ("PH", "Philippines"),
    ("PK", "Pakistan"),
    ("PL", "Poland"),
    ("PM", "Saint Pierre and Miquelon"),
    ("PN", "Pitcairn"),
    ("PR", "Puerto Rico"),
    ("PS", "Palestine, State of"),
    ("PT", "Portugal"),
    ("PW", "Palau"),
    ("PY", "Paraguay"),
    ("QA", "Qatar"),
    ("RE", "Réunion"),
    ("RO", "Romania"),
    ("RS", "Serbia"),
    ("RU", "Russian Federation"),
    ("RW", "Rwanda"),
    ("SA", "Saudi Arabia"),
    ("SB", "Solomon Islands"),
    ("SC", "Seychelles"),
    ("SD", "Sudan"),
    ("SE", "Sweden"),
    ("SG", "Singapore"),
    ("SH", "Saint Helena, Ascension and Tristan da Cunha"),
    ("SI", "Slovenia"),
    ("SJ", "Svalbard and Jan Mayen"),
    ("SK", "Slovakia"),
    ("SL", "Sierra Leone"),
    ("SM", "San Marino"),
    ("SN", "Senegal"),
    ("SO", "Somalia"),
    ("SR", "Suriname"),
    ("SS", "South Sudan"),
    ("ST", "Sao Tome and Principe"),
    ("SV", "El Salvador"),
    ("SX", "Sint Maarten (Dutch part)"),
    ("SY", "Syrian Arab Republic"),
    ("SZ", "Eswatini"),
    ("TC", "Turks and Caicos Islands"),
    ("TD", "Chad"),
    ("TF", "French Southern Territories"),
    ("TG", "Togo"),
    ("TH", "Thailand"),
    ("TJ", "Tajikistan"),
    ("TK", "Tokelau"),
    ("TL", "Timor-Leste"),
    ("TM", "Turkmenistan"),
    ("TN", "Tunisia"),
    ("TO", "Tonga"),
    ("TR", "Türkiye"),
    ("TT", "Trinidad and Tobago"),
    ("TV", "Tuvalu"),
    ("TW", "Taiwan, Province of China"),
    ("TZ", "Tanzania, United Republic of"),
    ("UA", "Ukraine"),
    ("UG", "Uganda"),
    ("UM", "United States Minor Outlying Islands"),
    ("US", "United States of America"),
    ("UY", "Uruguay"),
    ("UZ", "Uzbekistan"),
    ("VA", "Holy See"),
    ("VC", "Saint Vincent and the Grenadines"),
    ("VE", "Venezuela, Bolivarian Republic of"),
    ("VG", "Virgin Islands (British)"),
    ("VI", "Virgin Islands (U.S.)"),
    ("VN", "Viet Nam"),
    ("VU", "Vanuatu"),
    ("WF", "Wallis and Futuna"),
    ("WS", "Samoa"),
    ("YE", "Yemen"),
    ("YT", "Mayotte"),
    ("ZA", "South Africa"),
    ("ZM", "Zambia"),
    ("ZW", "Zimbabwe"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_every_assigned_code() {
        assert_eq!(COUNTRIES.len(), 249);
    }

    #[test]
    fn table_is_sorted_and_unique() {
        for pair in COUNTRIES.windows(2) {
            assert!(
                pair[0].0 < pair[1].0,
                "country table must be strictly sorted"
            );
        }
    }

    #[test]
    fn substitute_prefixes_are_sorted() {
        for pair in ISIN_SUBSTITUTE_PREFIXES.windows(2) {
            assert!(
                pair[0] < pair[1],
                "substitute prefixes must be strictly sorted"
            );
        }
    }

    #[test]
    fn is_iso_country_accepts_known_codes() {
        for code in ["US", "GB", "FR", "DE", "CH", "JP", "KR", "AF", "ZW"] {
            assert!(is_iso_country(code), "{code} should be a country");
        }
    }

    #[test]
    fn is_iso_country_rejects_non_codes() {
        for code in ["ZZ", "QQ", "", "U", "USA", "us", "Us"] {
            assert!(!is_iso_country(code), "{code} should not be a country");
        }
    }

    #[test]
    fn substitutes_are_not_iso_countries() {
        for code in ISIN_SUBSTITUTE_PREFIXES {
            assert!(!is_iso_country(code), "{code} must not be an ISO country");
        }
    }

    #[test]
    fn country_name_round_trips() {
        assert_eq!(country_name("FR"), Some("France"));
        assert!(country_name("US").is_some());
        assert_eq!(country_name("ZZ"), None);
        assert_eq!(country_name("xs"), None);
    }

    #[test]
    fn is_isin_prefix_accepts_countries_and_substitutes() {
        assert!(is_isin_prefix("US"));
        assert!(is_isin_prefix("DE"));
        for code in ISIN_SUBSTITUTE_PREFIXES {
            assert!(is_isin_prefix(code), "{code} should be a valid ISIN prefix");
        }
    }

    #[test]
    fn is_isin_prefix_rejects_unknown() {
        for code in ["ZZ", "QQ", "", "xs", "eu"] {
            assert!(!is_isin_prefix(code), "{code} should not be an ISIN prefix");
        }
    }

    #[test]
    fn isin_prefix_name_describes_iso_countries() {
        assert_eq!(isin_prefix_name("US"), Some("United States of America"));
        assert_eq!(isin_prefix_name("FR"), Some("France"));
        assert_eq!(isin_prefix_name("DE"), country_name("DE"));
    }

    #[test]
    fn isin_prefix_name_describes_substitute_prefixes() {
        assert_eq!(
            isin_prefix_name("XS"),
            Some("Internationally cleared securities")
        );
        assert_eq!(isin_prefix_name("EU"), Some("European Union institutions"));
        assert_eq!(isin_prefix_name("XK"), Some("Kosovo"));
        for code in ["XA", "XB", "XC", "XD", "XF", "QS", "QT"] {
            assert_eq!(isin_prefix_name(code), Some("ISIN substitute prefix"));
        }
    }

    #[test]
    fn isin_prefix_name_rejects_unknown_and_case_sensitive() {
        assert_eq!(isin_prefix_name("ZZ"), None);
        assert_eq!(isin_prefix_name(""), None);
        assert_eq!(isin_prefix_name("xs"), None);
        assert_eq!(isin_prefix_name("us"), None);
    }
}
