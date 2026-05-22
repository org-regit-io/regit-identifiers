// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Quickstart example for regit-identifiers.
//!
//! A guided tour of the crate: parse and inspect each securities identifier
//! with a real instrument (ISIN, CUSIP / CINS, SEDOL, LEI, FIGI, BIC, MIC,
//! CFI, WKN, VALOR), recompute check digits with the `checkdigit` core,
//! convert ISIN ⇄ CUSIP / SEDOL / VALOR through the `convert` layer,
//! auto-detect an unlabelled identifier with `SecurityId::detect`, and look a
//! market up in the embedded ISO 10383 MIC registry.

use regit_identifiers::checkdigit;
use regit_identifiers::convert::{
    cusip_to_isin, isin_to_cusip, isin_to_sedol, isin_to_valor, sedol_to_isin, valor_to_isin,
};
use regit_identifiers::detect::{IdentifierKind, SecurityId};
use regit_identifiers::{Bic, Cfi, Cusip, Figi, Isin, Lei, Mic, Sedol, Valor, Wkn};

// A guided tour is one long, deliberately linear `main` — fourteen labelled
// sections that read top to bottom.
#[allow(clippy::too_many_lines)]
fn main() {
    // ── 1. ISIN — ISO 6166, the primary key of a security ───────────────
    let isin = Isin::parse("US0378331005").expect("Apple Inc. ISIN");
    println!("ISIN  {}  (Apple Inc.)", isin.as_str());
    println!("  country = {}", isin.country_code());
    println!("  NSIN    = {}", isin.nsin());
    println!("  check   = {}", isin.check_digit());

    // ── 2. CUSIP / CINS — ANSI X9.6, the US / Canada national number ────
    let cusip = Cusip::parse("037833100").expect("Apple Inc. CUSIP");
    println!("\nCUSIP {}  (Apple Inc.)", cusip.as_str());
    println!("  issuer = {}", cusip.issuer());
    println!("  issue  = {}", cusip.issue());
    println!("  check  = {}", cusip.check_digit());
    println!("  CINS?  = {}", cusip.is_cins());

    // ── 3. SEDOL — London Stock Exchange Masterfile number ──────────────
    let sedol = Sedol::parse("0263494").expect("BAE Systems SEDOL");
    println!("\nSEDOL {}  (BAE Systems plc)", sedol.as_str());
    println!("  body            = {}", sedol.body());
    println!("  check           = {}", sedol.check_digit());
    println!("  legacy numeric? = {}", sedol.is_legacy_numeric());

    // ── 4. LEI — ISO 17442, the global legal-entity reference ───────────
    let lei = Lei::parse("5493001KJTIIGC8Y1R12").expect("Bloomberg Finance LEI");
    println!("\nLEI   {}  (Bloomberg Finance L.P.)", lei.as_str());
    println!("  LOU prefix   = {}", lei.lou_prefix());
    println!("  entity id    = {}", lei.entity_id());
    println!("  check digits = {}", lei.check_digits());

    // ── 5. FIGI — ANSI X9.145, the OpenFIGI instrument identifier ───────
    let figi = Figi::parse("BBG000BLNNH6").expect("IBM FIGI");
    println!("\nFIGI  {}  (IBM)", figi.as_str());
    println!("  provider prefix = {}", figi.provider_prefix());
    println!("  body            = {}", figi.body());
    println!("  check           = {}", figi.check_digit());
    println!("  Bloomberg?      = {}", figi.is_bloomberg());

    // ── 6. BIC — ISO 9362, the SWIFT address of an institution ──────────
    let bic = Bic::parse("DEUTDEFF500").expect("Deutsche Bank BIC");
    println!("\nBIC   {}  (Deutsche Bank, Frankfurt)", bic.as_str());
    println!("  institution = {}", bic.institution());
    println!("  country     = {}", bic.country_code());
    println!("  location    = {}", bic.location_code());
    println!(
        "  branch      = {}",
        bic.branch_code().unwrap_or("(primary office)")
    );

    // ── 7. MIC — ISO 10383, a market rather than a security ─────────────
    let mic = Mic::parse("XNAS").expect("Nasdaq MIC");
    println!("\nMIC   {}  (Nasdaq)", mic.as_str());
    println!("  prefix = {}", mic.prefix());
    println!("  suffix = {}", mic.suffix());

    // ── 8. CFI — ISO 10962, the classification of an instrument ─────────
    let cfi = Cfi::parse("ESVUFR").expect("equity CFI");
    println!("\nCFI   {}", cfi.as_str());
    println!(
        "  category   = {} ({})",
        cfi.category(),
        cfi.category_name()
    );
    println!("  group      = {}", cfi.group());
    println!("  attributes = {}", cfi.attributes());

    // ── 9. WKN — German national securities number ──────────────────────
    let wkn = Wkn::parse("766403").expect("Volkswagen WKN");
    println!("\nWKN   {}  (Volkswagen AG)", wkn.as_str());
    println!("  numeric? = {}", wkn.is_numeric());

    // ── 10. VALOR — Swiss national securities number ────────────────────
    let valor = Valor::parse("1213853").expect("a Swiss VALOR");
    println!("\nVALOR {}", valor.as_str());
    println!("  digits = {}", valor.len());
    println!("  value  = {}", valor.as_u64());

    // ── 11. The check-digit core — recompute, never trust ───────────────
    // Every parser above verified its identifier by recomputing the check
    // digit from the governing standard. The same algorithms are exposed
    // directly over the identifier body.
    println!("\nCheck-digit core (recomputed from each standard)");
    println!(
        "  ISIN  US037833100      -> {}",
        checkdigit::isin_check_digit("US037833100").expect("ISIN body")
    );
    println!(
        "  CUSIP 03783310         -> {}",
        checkdigit::cusip_check_digit("03783310").expect("CUSIP body")
    );
    println!(
        "  SEDOL 026349           -> {}",
        checkdigit::sedol_check_digit("026349").expect("SEDOL body")
    );
    println!(
        "  FIGI  BBG000BLNNH      -> {}",
        checkdigit::figi_check_digit("BBG000BLNNH").expect("FIGI body")
    );
    let lei_digits = checkdigit::lei_check_digits("5493001KJTIIGC8Y1R").expect("LEI body");
    println!(
        "  LEI   5493001KJTIIGC8Y1R -> {}{}",
        lei_digits[0], lei_digits[1]
    );

    // ── 12. Conversions — ISIN ⇄ CUSIP / SEDOL / VALOR ──────────────────
    // The ISIN is the international wrapper around a national number. The
    // conversions extract that number and re-wrap it, recomputing — never
    // reusing — the target's independent check digit.
    println!("\nConversions");

    let extracted_cusip = isin_to_cusip(&isin).expect("US ISIN embeds a CUSIP");
    let rebuilt_isin = cusip_to_isin(&extracted_cusip, "US").expect("CUSIP -> ISIN");
    println!(
        "  {} -> CUSIP {} -> {}  (round-trip: {})",
        isin.as_str(),
        extracted_cusip.as_str(),
        rebuilt_isin.as_str(),
        rebuilt_isin == isin,
    );

    let gb_isin = Isin::parse("GB0002634946").expect("BAE Systems ISIN");
    let extracted_sedol = isin_to_sedol(&gb_isin).expect("GB ISIN embeds a SEDOL");
    let rebuilt_gb = sedol_to_isin(&extracted_sedol, "GB").expect("SEDOL -> ISIN");
    println!(
        "  {} -> SEDOL {} -> {}  (round-trip: {})",
        gb_isin.as_str(),
        extracted_sedol.as_str(),
        rebuilt_gb.as_str(),
        rebuilt_gb == gb_isin,
    );

    let ch_isin = Isin::parse("CH0012138530").expect("a Swiss ISIN");
    let extracted_valor = isin_to_valor(&ch_isin).expect("CH ISIN embeds a VALOR");
    let rebuilt_ch = valor_to_isin(&extracted_valor, "CH").expect("VALOR -> ISIN");
    println!(
        "  {} -> VALOR {} -> {}  (round-trip: {})",
        ch_isin.as_str(),
        extracted_valor.as_str(),
        rebuilt_ch.as_str(),
        rebuilt_ch == ch_isin,
    );

    // ── 13. Auto-detection — recognising an unlabelled identifier ───────
    // Reference data rarely arrives labelled; `SecurityId::detect` decides
    // which checksum-bearing kind a raw string is.
    println!("\nAuto-detection (SecurityId::detect)");
    for raw in [
        "US0378331005",
        "5493001KJTIIGC8Y1R12",
        "BBG000BLNNH6",
        "037833100",
        "0263494",
        "DEUTDEFF",
        "not-an-identifier",
    ] {
        match SecurityId::detect(raw) {
            Some(id) => println!("  {:<22} -> {:?}", raw, id.kind()),
            None => println!("  {raw:<22} -> (unrecognised)"),
        }
    }
    // The structural-only kinds carry no check digit and are not detected.
    assert_eq!(
        SecurityId::detect("US0378331005").map(|id| id.kind()),
        Some(IdentifierKind::Isin)
    );

    // ── 14. MIC registry — structural validity is not existence ─────────
    // `XNAS` is well-formed and a real market; `ZZZZ` is well-formed but
    // names nothing. Only the embedded ISO 10383 registry can tell them
    // apart.
    println!("\nMIC registry lookup");
    #[cfg(feature = "mic-registry")]
    {
        use regit_identifiers::mic_registry::SNAPSHOT_DATE;
        println!("  ISO 10383 snapshot date: {SNAPSHOT_DATE}");
        for code in ["XNAS", "XLON", "XPAR", "ZZZZ"] {
            match Mic::parse(code).expect("structurally valid MIC").lookup() {
                Some(entry) => println!(
                    "  {:<5} -> {} — {}, {} [{:?}]",
                    code, entry.name, entry.city, entry.country, entry.status,
                ),
                None => println!("  {code:<5} -> well-formed, but not in the registry"),
            }
        }
    }
    #[cfg(not(feature = "mic-registry"))]
    {
        println!("  (built with --no-default-features: registry not embedded)");
        println!(
            "  MIC structure still validates: {}",
            Mic::validate("XNAS").is_ok()
        );
    }
}
