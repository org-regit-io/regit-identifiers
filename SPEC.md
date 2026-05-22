<!-- Copyright 2026 Regit.io — Nicolas Koenig -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# SPEC.md — regit-identifiers

> The reference specification for every identifier this crate validates. For
> each identifier it states the structure (segment offsets, lengths, character
> sets), the check-digit algorithm where one exists, one worked example
> computed by hand against a real instrument, and the governing standard cited
> by number.
>
> This document is the public, citable distillation of the crate's internal
> verified algorithm reference. The crate is the executable form of this
> specification — every rule stated here is enforced by the `src/` module named
> in its heading, and a parser never trusts a supplied check digit: it
> recomputes it from the standard and compares.

---

## Table of contents

1. [Conventions](#conventions)
2. [The check-digit core — `src/checkdigit.rs`](#the-check-digit-core--srccheckdigitrs)
3. [ISIN — ISO 6166 — `src/isin.rs`](#isin--iso-6166--srcisinrs)
4. [CUSIP — ANSI X9.6 — `src/cusip.rs`](#cusip--ansi-x96--srccusiprs)
5. [CINS — CUSIP International Numbering System — `src/cusip.rs`](#cins--cusip-international-numbering-system--srccusiprs)
6. [SEDOL — London Stock Exchange — `src/sedol.rs`](#sedol--london-stock-exchange--srcsedolrs)
7. [LEI — ISO 17442 / ISO 7064 — `src/lei.rs`](#lei--iso-17442--iso-7064--srcleirs)
8. [BIC — ISO 9362 — `src/bic.rs`](#bic--iso-9362--srcbicrs)
9. [MIC — ISO 10383 — `src/mic.rs`](#mic--iso-10383--srcmicrs)
10. [FIGI — ANSI X9.145 — `src/figi.rs`](#figi--ansi-x9145--srcfigirs)
11. [CFI — ISO 10962 — `src/cfi.rs`](#cfi--iso-10962--srccfirs)
12. [WKN — Wertpapierkennnummer — `src/wkn.rs`](#wkn--wertpapierkennnummer--srcwknrs)
13. [VALOR — Valorennummer — `src/valor.rs`](#valor--valorennummer--srcvalorrs)
14. [Cross-identifier conversions — `src/convert.rs`](#cross-identifier-conversions--srcconvertrs)
15. [`SecurityId` auto-detection — `src/detect.rs`](#securityid-auto-detection--srcdetectrs)
16. [Standards index](#standards-index)

---

## Conventions

The conventions below hold for every identifier in this document.

- **Character positions.** Offsets are stated two ways. A `[a..b]` range is a
  zero-based, half-open byte slice — `[0..2]` is the first two characters.
  "Character *n*" is the one-based human position — character 1 is `[0..1]`.
  Both appear; the worked examples use the one-based form.
- **Character sets.** `[0-9]` is the ASCII decimal digits; `[A-Z]` is the ASCII
  upper-case letters; `[A-Z0-9]` is their union. Every identifier is
  **upper-case only** — a lower-case letter is rejected, never folded. A
  non-ASCII character is rejected at the character-set check before any
  arithmetic runs; nothing in this crate panics on attacker-controlled input.
- **Letter values.** Two distinct letter-to-number mappings are used and must
  not be confused:
  - *Expansion* — a letter becomes the two-digit number `10 + (c - 'A')`, so
    `A = 10`, `B = 11`, …, `Z = 35`. Used by ISIN and LEI.
  - *Alphanumeric value* — a letter takes the single value `10 + (c - 'A')`
    (same range `10..=35`, but one value, not two digits). Used by CUSIP,
    SEDOL, and FIGI.
- **Check-digit formula.** Every weighted-sum scheme here finishes with
  `check = (10 - (S mod 10)) mod 10`, where `S` is the scheme's running sum.
  The outer `mod 10` maps the `10 - 0 = 10` case back to `0`.
- **Verification, not trust.** A parser recomputes the check digit(s) from the
  identifier body and compares against the supplied value. A mismatch is
  rejected — a wrong check digit is the worst possible silent failure and this
  crate never produces one.

---

## The check-digit core — `src/checkdigit.rs`

Five of the eleven identifiers carry a check digit. Each uses a different
algorithm, and three details are load-bearing and easy to get subtly wrong: the
**scan direction**, the **letter mapping**, and whether a two-digit weighted
product is **folded** to its digit sum. The table below states all three for
every scheme; the per-identifier sections give the full procedure.

```text
Identifier  Scan direction   Letter mapping       Two-digit product
─────────── ──────────────── ──────────────────── ─────────────────────────
ISIN        right-to-left    expand to two digits  folded (Luhn)
CUSIP       left-to-right    single value 10..35   folded: floor(p/10)+p mod 10
SEDOL       left-to-right    single value 10..35   NOT folded
LEI         whole string     expand to two digits  n/a — MOD 97-10
FIGI        right-to-left    single value 10..35   folded: sum of decimal digits
```

The `checkdigit` module is public: each algorithm is also callable directly on
an identifier *body* (the identifier without its check digit), returning the
digit the standard prescribes.

```text
luhn_checksum(digits)   -> 0..=9    Luhn mod-10 of a pure-digit string
isin_check_digit(body)  -> char     ISO 6166,  11-char body
cusip_check_digit(body) -> char     ANSI X9.6,  8-char body
sedol_check_digit(body) -> char     LSE SEDOL,  6-char body
lei_check_digits(body)  -> [char;2] ISO 7064 MOD 97-10, 18-char body
figi_check_digit(body)  -> char     ANSI X9.145, 11-char body
```

---

## ISIN — ISO 6166 — `src/isin.rs`

The International Securities Identification Number is the globally recognised
primary key of a security.

### Structure

```text
Length: 12 characters, fixed.

  U S 0 3 7 8 3 3 1 0 0 5
  └┬┘ └────┬────┘ │
   │       │       └ check digit  [11]      one digit            [0-9]
   │       └───────── NSIN         [2..11]   nine characters      [A-Z0-9]
   └───────────────── country      [0..2]    ISO 3166-1 or prefix [A-Z]

Segment      Offset    Length  Character set
──────────── ───────── ─────── ─────────────────────────────────────────
country      [0..2]    2       a recognised ISIN country prefix
NSIN         [2..11]   9       [A-Z0-9]
check digit  [11..12]  1       [0-9]
```

The **country prefix** is an ISO 3166-1 alpha-2 code or one of the ISIN
substitute prefixes — `EU`, `QS`, `QT`, `XA`, `XB`, `XC`, `XD`, `XF`, `XK`,
`XS`. The substitutes name securities with no single national home (`XS` for
internationally-cleared securities, `EU` for European Union institutions) and
are *not* ISO 3166-1 countries; an ISIN parser must accept them or it wrongly
rejects valid ISINs. The **NSIN** (National Securities Identifying Number) is
the local identifier, left-padded into the nine-character field.

### Check-digit algorithm

The ISIN check digit is a Luhn mod-10, but the Luhn weights are assigned to an
**expanded** digit string — parity is decided *after* expansion, not over the
original 11 characters.

```text
Input: the 11-character body (country prefix + NSIN).

1. Expand the body, processing characters right-to-left. A digit emits itself,
   one decimal digit. A letter emits the two decimal digits of 10 + (c - 'A')
   — the tens digit to the left of the units digit.
2. Apply the Luhn weighting to the expanded digit string, right-to-left: the
   rightmost expanded digit has weight 2, then weights alternate 1, 2, 1, ...
3. For each expanded digit d with weight w, let p = d * w. If p >= 10, fold it
   to p - 9 (the sum of its two digits). Add the result to the running sum S.
4. check digit = (10 - (S mod 10)) mod 10.
```

### Worked example — Apple Inc., `US0378331005`

```text
Body = "US037833100".

Expand each character (U = 30, S = 28; digits unchanged):
  U -> 3 0   S -> 2 8   0 -> 0   3 -> 3   7 -> 7   8 -> 8
  3 -> 3   3 -> 3   1 -> 1   0 -> 0   0 -> 0
Expanded digit string (left to right): 3 0 2 8 0 3 7 8 3 3 1 0 0

Apply Luhn weights right-to-left (rightmost weight 2):
  digit   3 0 2 8 0 3 7 8 3 3 1 0 0
  weight  2 1 2 1 2 1 2 1 2 1 2 1 2
  p       6 0 4 8 0 3 14 8 6 3 2 0 0
  folded  6 0 4 8 0 3  5 8 6 3 2 0 0

  S = 6+0+4+8+0+3+5+8+6+3+2+0+0 = 45

check digit = (10 - (45 mod 10)) mod 10 = (10 - 5) mod 10 = 5.
```

The check digit is **5**, so the full ISIN is `US0378331005`. Verified.

Other verified ISINs: `GB0002634946` (BAE Systems plc), `DE000BAY0017`
(Bayer AG).

### Governing standard

ISO 6166, *Securities and related financial instruments — International
securities identification number (ISIN)*. Country prefixes are governed by
ISO 3166-1 alpha-2 plus the registered ISIN substitute prefixes.

---

## CUSIP — ANSI X9.6 — `src/cusip.rs`

The CUSIP (Committee on Uniform Securities Identification Procedures) number is
the national securities identifier of the United States and Canada.

### Structure

```text
Length: 9 characters, fixed.

  0 3 7 8 3 3 1 0 0
  └────┬────┘ └┬┘ │
       │       │   └ check digit  [8]      one digit         [0-9]
       │       └───── issue        [6..8]   two characters    [A-Z0-9*@#]
       └───────────── issuer       [0..6]   six characters    [A-Z0-9*@#]

Segment      Offset    Length  Character set
──────────── ───────── ─────── ──────────────────────────────────────────
issuer       [0..6]    6       [A-Z0-9*@#]
issue        [6..8]    2       [A-Z0-9*@#]
check digit  [8..9]    1       [0-9]
```

The body alphabet is the digits, the upper-case letters (`I` and `O` *are*
legal in a CUSIP), and the three special characters `*`, `@`, `#`. The
**issuer** segment names the issuing entity; the **issue** segment names a
specific security of that issuer.

### Check-digit algorithm

The CUSIP check digit is the "modulus 10 double add double" of ANSI X9.6.

```text
Input: the 8-character body (issuer + issue).

Character values: a digit is its own value; a letter is 10 + (c - 'A');
the special characters extend the alphabet — * = 36, @ = 37, # = 38.

1. Process the body left-to-right with one-based positions. Odd positions
   (1, 3, 5, 7) take weight 1; even positions (2, 4, 6, 8) take weight 2.
2. For each character of value v at weight w, let p = v * w. Fold p to
   floor(p / 10) + (p mod 10) and add the result to the running sum S.
   (p can reach 76 — for example 38 * 2 — so folding is always to the sum of
   the product's own decimal digits.)
3. check digit = (10 - (S mod 10)) mod 10.
```

### Worked example — Apple Inc., `037833100`

```text
Body = "03783310".

position  1 2 3 4 5 6 7 8
char      0 3 7 8 3 3 1 0
value     0 3 7 8 3 3 1 0
weight    1 2 1 2 1 2 1 2
p         0 6 7 16 3 6 1 0
folded    0 6 7  7 3 6 1 0      (16 -> 1 + 6 = 7)

S = 0+6+7+7+3+6+1+0 = 30

check digit = (10 - (30 mod 10)) mod 10 = (10 - 0) mod 10 = 0.
```

The check digit is **0**, so the full CUSIP is `037833100`. Verified.

Other verified CUSIPs: `594918104` (Microsoft Corp.), `38259P508`
(Alphabet Inc.).

### Governing standard

ANSI X9.6, *Financial Services — CUSIP Numbering System*, administered by
CUSIP Global Services.

---

## CINS — CUSIP International Numbering System — `src/cusip.rs`

A CINS number is structurally a CUSIP — same 9-character length, same body
alphabet, **same check-digit algorithm** — issued for non-US/non-Canadian
securities. It is represented by the same `Cusip` type.

### Distinguishing rule

```text
A CINS number begins with a LETTER.
A domestic CUSIP begins with a DIGIT.

This is the only structural difference; the check digit is computed by the
identical ANSI X9.6 procedure stated above.
```

The leading letter designates the issuing region:

```text
A Austria       B Belgium        C Canada         D Germany
E Spain         F France         G United Kingdom H Switzerland
J Japan         K Denmark        L Luxembourg     M Middle East
N Netherlands   P South America  Q Australia      R Norway
S South Africa  T Italy          U United States  V Africa-Other
W Sweden        X Europe-Other   Y Asia

Letters I, O, and Z are not assigned a region.
```

`Cusip::is_cins` reports whether the first character is a letter;
`Cusip::cins_region` maps that letter to its region (returning `None` for a
domestic CUSIP or an unassigned letter).

### Governing standard

CINS is administered by CUSIP Global Services under the same ANSI X9.6 scheme
as the CUSIP.

---

## SEDOL — London Stock Exchange — `src/sedol.rs`

The SEDOL (Stock Exchange Daily Official List) number is the national
securities identifier for instruments listed in the United Kingdom and Ireland.

### Structure

```text
Length: 7 characters, fixed.

  0 2 6 3 4 9 4
  └────┬────┘ │
       │       └ check digit  [6]      one digit                       [0-9]
       └──────── body          [0..6]   six characters, digits + consonants

Segment      Offset    Length  Character set
──────────── ───────── ─────── ──────────────────────────────────────────
body         [0..6]    6       digits [0-9] and consonants (B-Z, no vowels)
check digit  [6..7]    1       [0-9]
```

The body alphabet is the digits and the **consonants** — the vowels `A`, `E`,
`I`, `O`, `U` are never used. A vowel in the body is rejected as an invalid
character. SEDOLs issued before the 2004 switch to an alphanumeric scheme have
a purely numeric body; `Sedol::is_legacy_numeric` reports this.

### Check-digit algorithm

The SEDOL check digit is a fixed-weight modular sum. Unlike ISIN, CUSIP, and
FIGI, it does **not** fold a two-digit weighted product to its digit sum.

```text
Input: the 6-character body.

Character values: a digit is its own value; a consonant is 10 + (c - 'A')
(so B = 11, C = 12, ..., Z = 35).

1. Apply the fixed weight vector [1, 3, 1, 7, 3, 9] to the six body
   characters, left-to-right: character 1 takes weight 1, character 2 takes
   weight 3, and so on.
2. S = sum over i of value_i * weight_i. The products are summed directly —
   there is NO digit-folding step.
3. check digit = (10 - (S mod 10)) mod 10.
```

### Worked example — BAE Systems plc, `0263494`

```text
Body = "026349".

position  1 2 3 4 5 6
char      0 2 6 3 4 9
value     0 2 6 3 4 9
weight    1 3 1 7 3 9
product   0 6 6 21 12 81

S = 0 + 6 + 6 + 21 + 12 + 81 = 126

check digit = (10 - (126 mod 10)) mod 10 = (10 - 6) mod 10 = 4.
```

The check digit is **4**, so the full SEDOL is `0263494`. Verified.

Other verified SEDOL: `0540528` (a legacy numeric SEDOL).

### Governing standard

The SEDOL Masterfile service of the London Stock Exchange.

---

## LEI — ISO 17442 / ISO 7064 — `src/lei.rs`

The Legal Entity Identifier is the globally unique reference code for a legal
entity that participates in a financial transaction.

### Structure

```text
Length: 20 characters, fixed.

  5 4 9 3 0 0 1 K J T I I G C 8 Y 1 R 1 2
  └──┬──┘ └┬┘ └──────┬──────┘ └┬┘
     │     │         │         └ check digits [18..20]  two digits   [0-9]
     │     │         └─────────── entity ID   [6..18]   twelve chars [A-Z0-9]
     │     └───────────────────── reserved    [4..6]    the literal "00"
     └─────────────────────────── LOU prefix  [0..4]    four chars   [A-Z0-9]

Segment      Offset    Length  Character set
──────────── ───────── ─────── ──────────────────────────────────────────
LOU prefix   [0..4]    4       [A-Z0-9]
reserved     [4..6]    2       the literal "00" (any other value rejected)
entity ID    [6..18]   12      [A-Z0-9]
check digits [18..20]  2       [0-9]
```

The **LOU prefix** identifies the Local Operating Unit that issued the
identifier. Positions 5–6 are a **reserved** field fixed by the standard to the
literal `00`; any other value is a structural violation. The **entity ID** is
the LOU-assigned unique reference. (ISO 17442 §5 calls this segment the
*entity-specific part*; this crate's accessor is `Lei::entity_id`.)

### Check-digit algorithm

The two LEI check digits are an ISO/IEC 7064 MOD 97-10 system — the same scheme
as the IBAN.

```text
Input: the 18-character body (LOU prefix + reserved + entity part).

1. Form the integer M: expand the 18-character body followed by the literal
   "00" — a digit contributes one decimal place, a letter contributes the two
   digits of 10 + (c - 'A'). M is the resulting ~38-digit number.
2. check digits = 98 - (M mod 97), written as two digits, zero-padded into
   the range 01..98.

Validation of a complete 20-character LEI: expand all 20 characters into one
integer N; the LEI is valid iff N mod 97 == 1.

The modulus is computed by a STREAMING recurrence — the wide integer is never
formed:
    for a digit d:           acc = (acc * 10  + d) mod 97
    for an expanded letter v: acc = (acc * 100 + v) mod 97   (v in 10..=35)
The accumulator is a residue below 97, so the largest intermediate value is
96 * 100 + 35 = 9635 — well within a 32-bit integer.
```

### Worked example — Bloomberg Finance L.P., `5493001KJTIIGC8Y1R12`

```text
Full LEI = "5493001KJTIIGC8Y1R12"  (body "5493001KJTIIGC8Y1R", check "12").

Validation form. Expand each of the 20 characters in place — digits unchanged,
letters becoming the two-digit number 10 + (c - 'A'):

  5 4 9 3 0 0 1 K  J  T  I  I  G  C  8 Y  1 R  1 2
  5 4 9 3 0 0 1 20 19 29 18 18 16 12 8 34 1 27 1 2

Concatenating yields the ~38-digit integer

  N = 549300012019291818161283413127 12

reduced by the streaming recurrence

      for a digit d:           acc = (acc * 10  + d) mod 97
      for an expanded letter v: acc = (acc * 100 + v) mod 97

so the wide integer is never formed. The ISO 7064 MOD 97-10 acceptance
condition is

  N mod 97 == 1.

For this LEI the recurrence terminates with acc = 1, so the check digits "12"
are exactly the pair that makes the congruence hold. Equivalently, computing
the check digits from the 18-character body alone uses the same recurrence
followed by the literal "00", then check = 98 - acc; for this body that
yields the pair (1, 2).
```

The check digits are **12**, so the full LEI is `5493001KJTIIGC8Y1R12`.
Verified: `N mod 97 == 1`.

### Governing standard

ISO 17442, *Financial services — Legal entity identifier (LEI)*. The check
digits follow ISO/IEC 7064, *Information technology — Security techniques —
Check character systems*, MOD 97-10 system.

---

## BIC — ISO 9362 — `src/bic.rs`

The Business Identifier Code (the SWIFT address) identifies a bank or other
institution on the SWIFT network. It carries **no check digit** — validation is
structural.

### Structure

```text
Length: 8 or 11 characters. No other length is permitted.

  D E U T D E F F 5 0 0
  └──┬──┘ └┬┘ └┬┘ └─┬─┘
    │      │   │     └ branch       [8..11]  three [A-Z0-9], 11-char BIC only
    │      │   └─────── location    [6..8]   two   [A-Z0-9]
    │      └─────────── country     [4..6]   ISO 3166-1 alpha-2 letters
    └────────────────── institution [0..4]   four  [A-Z]

Segment      Offset    Length  Character set
──────────── ───────── ─────── ──────────────────────────────────────────
institution  [0..4]    4       [A-Z]
country      [4..6]    2       ISO 3166-1 alpha-2 (must be a real code)
location     [6..8]    2       [A-Z0-9]
branch       [8..11]   3       [A-Z0-9]  (present only in an 11-char BIC)

An 8-character BIC is an institution's primary office; an 11-character BIC
appends an explicit 3-character branch code.
```

Characters 1–6 (institution and country) must be **letters only**. The country
segment must be a recognised ISO 3166-1 alpha-2 code. The location segment
carries a convention in its second character — `0` marks a test/training BIC,
`1` marks a passive SWIFT participant — exposed by `Bic::is_test_bic` and
`Bic::is_passive`.

### Worked examples

```text
DEUTDEFF      institution DEUT, country DE, location FF, no branch.
              Length 8, characters 1-6 all letters, DE is a valid ISO
              country -> structurally valid.

DEUTDEFF500   institution DEUT, country DE, location FF, branch 500.
              Length 11 -> structurally valid; branch code present.

CHASUS33      institution CHAS, country US, location 33, no branch.
              Length 8, US is a valid ISO country -> structurally valid.
```

There is no check digit, so a BIC is verified purely against length, the
per-segment character set, and the ISO 3166-1 country code.

### Governing standard

ISO 9362, *Banking — Banking telecommunication messages — Business identifier
code (BIC)*. Country codes follow ISO 3166-1 alpha-2.

---

## MIC — ISO 10383 — `src/mic.rs`

The Market Identifier Code names a trading venue — an exchange, a multilateral
trading facility, or another market — rather than a security. It carries **no
check digit**.

### Structure

```text
Length: 4 characters, fixed.

  X N A S
  │ └─┴─┘
  │   │
  │   └─── market suffix  [1..4]  three characters [A-Z0-9]
  └─────── leading char   [0]     one upper-case letter [A-Z]

Segment        Offset    Length  Character set
────────────── ───────── ─────── ─────────────────────────────────
leading char   [0..1]    1       [A-Z]
market suffix  [1..4]    3       [A-Z0-9]
```

ISO 10383 distinguishes an **operating MIC**, which identifies a market
operator, from a **segment MIC**, which names a sub-market and references its
operating MIC.

### Structural validity versus registry membership

Structural validity is necessary but **not sufficient**: `ZZZZ` is a
well-formed MIC yet identifies no real market. True validity is membership in
the published ISO 10383 registry. This crate embeds a dated snapshot of that
registry behind the default `mic-registry` feature:

```text
Mic::parse             structural validity only (length, character set).
Mic::parse_registered  structural validity AND presence in the embedded
                       ISO 10383 snapshot.
Mic::is_registered     reports embedded-registry membership.
Mic::lookup            returns the MicEntry (operating MIC, operator name,
                       country, city, status) for a registered code.
```

The embedded snapshot records its source date as `SNAPSHOT_DATE`; an auditor
should cite that date when reporting MIC validity, because the registry is
revised continually by the ISO 10383 Registration Authority.

### Worked examples

```text
XNAS   leading letter X, suffix NAS  -> structurally valid; registered
       (Nasdaq, United States).
XLON   leading letter X, suffix LON  -> structurally valid; registered
       (London Stock Exchange).
XPAR   leading letter X, suffix PAR  -> structurally valid; registered
       (Euronext Paris).
```

### Governing standard

ISO 10383, *Securities and related financial instruments — Codes for
exchanges and market identification (MIC)*.

---

## FIGI — ANSI X9.145 — `src/figi.rs`

The Financial Instrument Global Identifier is a permanent, currency- and
exchange-aware identifier for a financial instrument, issued through the
OpenFIGI programme.

### Structure

```text
Length: 12 characters, fixed.

  B B G 0 0 0 B L N N H 6
  └┬┘ │ └────┬─────┘ │
   │  │      │        └ check digit  [11]      one digit  [0-9]
   │  │      └───────── body         [3..11]   eight chars, digits/consonants
   │  └──────────────── literal 'G'  [2]       always the letter G
   └─────────────────── provider     [0..2]    two upper-case consonants

Segment        Offset    Length  Character set
────────────── ───────── ─────── ──────────────────────────────────────────
provider       [0..2]    2       two upper-case consonants (no vowels)
literal 'G'    [2..3]    1       always the letter G
body           [3..11]   8       digits [0-9] and consonants (no vowels)
check digit    [11..12]  1       [0-9]
```

The **provider prefix** is two upper-case consonants and must **not** be one of
`BS`, `BM`, `GG`, `GB`, `GH`, `KY`, `VG` — those would collide with ISIN
country codes. Bloomberg-issued FIGIs use the prefix `BBG`. Character 3 is
always the literal `G`. The body excludes vowels, so a FIGI is never confused
with a word.

### Check-digit algorithm

The FIGI check digit is a modulus-10 double-add-double scanned right-to-left,
but — unlike a plain Luhn — the **rightmost character carries weight 1, not 2**.

```text
Input: the 11-character body (provider + literal 'G' + body).

Character values: a digit is its own value; a consonant is 10 + (c - 'A')
(B = 11, C = 12, ..., Z = 35).

1. Process the body right-to-left. The rightmost character (one-based
   position 11) takes weight 1; the weight then alternates 2, 1, 2, ...
2. For each character of value v at weight w, let p = v * w. Add EVERY
   decimal digit of p to the running sum S — that is, floor(p / 10) +
   (p mod 10). (p cannot exceed 35 * 2 = 70.)
3. check digit = (10 - (S mod 10)) mod 10.
```

This is distinct from the ISIN algorithm — the doubling parity is shifted by
one — so the two are implemented separately and share no code.

### Worked example — IBM, `BBG000BLNNH6`

```text
Body = "BBG000BLNNH".

Character values (B = 11, G = 16, L = 21, N = 23, H = 17). Process the body
right-to-left; the rightmost character carries weight 1, then weights
alternate 2, 1, 2, ... Tabulated right-to-left:

  position (1-based)  11 10  9  8  7  6  5  4  3  2  1
  char (right-to-left) H  N  N  L  B  0  0  0  G  B  B
  value                17 23 23 21 11  0  0  0 16 11 11
  weight                1  2  1  2  1  2  1  2  1  2  1
  p = value * weight   17 46 23 42 11  0  0  0 16 22 11
  digit sum of p        8 10  5  6  2  0  0  0  7  4  2

  S = 8 + 10 + 5 + 6 + 2 + 0 + 0 + 0 + 7 + 4 + 2 = 44

check digit = (10 - (44 mod 10)) mod 10 = (10 - 4) mod 10 = 6.
```

The check digit is **6**, so the full FIGI is `BBG000BLNNH6`. Verified.

Other verified FIGI: `BBG000B9XRY4`.

### Governing standard

ANSI X9.145, *Financial Instrument Global Identifier (FIGI)*, with the OpenFIGI
specification of the Object Management Group.

---

## CFI — ISO 10962 — `src/cfi.rs`

The Classification of Financial Instruments code classifies *what kind* of
instrument something is, rather than which specific issue. It carries **no
check digit**.

### Structure

```text
Length: 6 characters, fixed. Every character is an upper-case letter [A-Z].

  E S V U F R
  │ │ └──┬──┘
  │ │    └──── attributes  [2..6]  four characters, instrument-specific
  │ └───────── group       [1]     a category-dependent sub-class
  └─────────── category    [0]     one of 14 ISO 10962 category letters

Segment      Offset    Length  Character set
──────────── ───────── ─────── ────────────────────────────────────────
category     [0..1]    1       one of: E C D R O F S H I J K L T M
group        [1..2]    1       [A-Z]
attributes   [2..6]    4       [A-Z]  ('X' = not applicable / not known)
```

The **category** (character 1) is the top-level class and must be one of the 14
ISO 10962 letters:

```text
E Equities                          C Collective investment vehicles
D Debt instruments                  R Entitlements (rights)
O Listed options                    F Futures
S Swaps                             H Non-listed and complex options
I Spot                              J Forwards
K Strategies                        L Financing
T Referential instruments           M Others
```

The **group** narrows the category; the four **attributes** further describe
the instrument, with `X` meaning "not applicable / not known".

### Scope of validation

`Cfi::parse` validates **structure and category only**: the exact length, the
all-`[A-Z]` character set, and that character 1 is a recognised category
letter. It deliberately does **not** validate the group character or the four
attributes against the per-category ISO 10962 tables. Those tables are large,
category-specific, and revised with each edition of the standard; validating
against an embedded snapshot would silently reject instruments classified under
a newer revision. The group and attributes are exposed verbatim through
accessors but left semantically unvalidated.

### Worked examples

```text
ESVUFR   category E (Equities), group S, attributes VUFR.
         6 letters, E is a valid category -> structurally valid.

DBFUGR   category D (Debt instruments), group B, attributes FUGR.
         6 letters, D is a valid category -> structurally valid.
```

### Governing standard

ISO 10962, *Securities and related financial instruments — Classification of
financial instruments (CFI) code*.

---

## WKN — Wertpapierkennnummer — `src/wkn.rs`

The WKN is the German national securities identifying number. It carries **no
check digit** and has no internal segments.

### Structure

```text
Length: 6 characters, fixed.

  A 1 E W W W
  └─────┬─────┘
        └ identifier  [0..6]  six characters [0-9A-Z], excluding I and O

Segment      Offset    Length  Character set
──────────── ───────── ─────── ──────────────────────────────────────────
identifier   [0..6]    6       [0-9A-Z] with the letters I and O excluded
```

Each character is an ASCII digit or an upper-case letter, with the two letters
`I` and `O` **excluded** — they are barred to avoid visual confusion with the
digits `1` and `0`. A literal `I` or `O` is rejected as an invalid character.

### Worked examples

```text
766403   six characters, all digits, none of them I or O -> valid.
A1EWWW   six characters [0-9A-Z], no I or O -> valid.
```

There is no check digit; validation is purely a length and character-set check.
`Wkn::is_numeric` reports whether every character is a digit.

### Governing standard

The Wertpapierkennnummer scheme of WM Datenservice, the German national
securities-numbering authority.

---

## VALOR — Valorennummer — `src/valor.rs`

The VALOR (Valorennummer) is the Swiss national securities identifying number.
It carries **no check digit**.

### Structure

```text
Length: 1 to 9 characters, variable.

  1 2 1 3 8 5 3
  └──────┬──────┘
         └ 1 to 9 digits [0-9]   no internal structure

Segment   Length   Character set
───────── ──────── ──────────────────────────────────
digits    1..=9    [0-9]  (ASCII decimal digits only)
```

A VALOR is a single run of between 1 and 9 ASCII decimal digits. It has no
segments and no check digit. A length of 0 or more than 9 is a structural
violation.

### Embedding in a Swiss ISIN

A Swiss (`CH`) or Liechtenstein (`LI`) ISIN embeds the VALOR directly: the
VALOR is left-padded with zeros to nine digits to form the NSIN, then the ISIN
prefix and check digit are added.

```text
VALOR 1213853  ->  NSIN "001213853"  ->  ISIN  CH0012138530
```

### Worked example

```text
1213853   seven ASCII digits, length within 1..9 -> valid.
          Left-padded to the 9-character NSIN it becomes "001213853",
          which yields the Swiss ISIN CH0012138530.
```

### Governing standard

The Valorennummer scheme of SIX Financial Information, the Swiss national
securities-identification authority.

---

## Cross-identifier conversions — `src/convert.rs`

An ISIN is the international wrapper around a *national* securities number (the
NSIN). For three jurisdictions that wrapper is exact and reversible. Every
conversion **recomputes** the target's check digit from its own standard and
**re-parses** the result through the target type's validator — the two
check-digit schemes involved are unrelated, so a conversion never reuses a
digit and never returns a wrong answer.

### Embedding rules

```text
US / CA ISIN   country prefix · 9-char NSIN · check    NSIN  IS  the CUSIP
GB / IE ISIN   country prefix · 00 + SEDOL  · check    NSIN  IS  00 + SEDOL
CH / LI ISIN   country prefix · 0…0 + VALOR · check    NSIN  IS  zero-padded VALOR
```

- **ISIN ↔ CUSIP** — a US or CA ISIN embeds a 9-character CUSIP verbatim as
  its NSIN. Extraction takes the 9 NSIN characters and re-parses them as a
  CUSIP (verifying the CUSIP check digit). Construction prefixes the country
  code and computes a fresh ISIN check digit. A non-US/CA ISIN has no defined
  CUSIP and returns `UnsupportedCountry`.
- **ISIN ↔ SEDOL** — a GB or IE ISIN embeds a 7-character SEDOL right-aligned
  in the 9-character NSIN, left-padded with the two literal characters `00`.
  Extraction strips the `00` and re-parses the remaining 7 characters as a
  SEDOL.
- **ISIN ↔ VALOR** — a CH or LI ISIN embeds a VALOR left-padded with zeros to
  nine digits. Extraction strips the leading zeros (keeping at least one digit,
  so an all-zero NSIN yields the VALOR `0`).
- **Generic build** — `build_isin(country, nsin)` left-zero-pads any national
  number into the 9-character NSIN field, computes the ISIN check digit, and
  re-parses the 12-character result through `Isin::parse`.

### Worked example — `US` CUSIP `037833100` to ISIN

```text
1. NSIN = the CUSIP, 9 characters     = "037833100"
2. body = country prefix + NSIN       = "US037833100"
3. ISIN check digit of that body      = 5     (see the ISIN worked example)
4. ISIN = body + check digit          = "US0378331005"
```

The CUSIP's own check digit is `0`; the resulting ISIN's is `5`. The two are
computed by unrelated algorithms — the conversion recomputes, it never carries
the CUSIP digit across.

### Errors

```text
UnsupportedCountry   the source country has no defined target for this
                     conversion (e.g. a CUSIP from a German ISIN).
NotConvertible       the inputs are malformed for the conversion (e.g. a
                     country prefix not exactly two characters, or an
                     overlong national number).
Validation(e)        the assembled result failed its target type's
                     validation; the inner ValidationError is carried.
```

### Governing standards

The conversions implement the embedding rules of ISO 6166 (ISIN), ANSI X9.6
(CUSIP), the LSE SEDOL scheme, and the SIX VALOR scheme.

---

## `SecurityId` auto-detection — `src/detect.rs`

Reference data rarely arrives labelled. `SecurityId::detect` takes a raw string
and returns the single identifier kind it is — or `None` when nothing fits.

### Detection order

Detection is **checksum-strength first**: a passing check digit is
high-confidence evidence, so kinds that carry one are tried before kinds that
do not. Each candidate is the strict `parse` of the corresponding type, so a
kind is reported only when the input is fully, structurally valid for it, check
digit included.

```text
Order tried:  LEI  ->  ISIN  ->  FIGI  ->  CUSIP  ->  SEDOL  ->  BIC  ->  MIC

Two ambiguities are resolved by this order:

  ISIN vs FIGI   both are 12 characters. ISIN is tried first. A genuine FIGI
                 is never a valid ISIN — it requires character 3 to be the
                 literal 'G' and forbids the seven ISIN-colliding provider
                 prefixes — so it falls through cleanly to the FIGI branch.

  CUSIP vs BIC   an 8-character string could be either. CUSIP carries a check
                 digit and is tried first; only a string that is not a valid
                 CUSIP reaches the BIC branch.
```

MIC is reported only when the `mic-registry` feature is enabled **and** the
string is a *registered* MIC — a structurally valid but unregistered code such
as `ZZZZ` is never auto-detected, because a MIC has no check digit and a bare
structural match is weak evidence.

### What is not auto-detected

Three structural-only kinds are **excluded** from auto-detection: **CFI**
(6 upper-case letters), **WKN** (6 alphanumeric characters), and **VALOR** (1
to 9 digits). They carry no check digit and overlap heavily with one another
and with other kinds, so detection could only guess. Parse them explicitly —
`Cfi::parse`, `Wkn::parse`, `Valor::parse` — when the kind is already known.

### Worked examples

```text
"5493001KJTIIGC8Y1R12"  ->  Lei    (20 chars, MOD 97-10 passes)
"US0378331005"          ->  Isin   (12 chars, Luhn passes; ISIN before FIGI)
"BBG000BLNNH6"          ->  Figi   (12 chars, char 3 = 'G', check passes)
"037833100"             ->  Cusip  ( 9 chars, X9.6 check passes)
"0263494"               ->  Sedol  ( 7 chars, weighted check passes)
"DEUTDEFF"              ->  Bic    ( 8 chars, ISO 9362 structure)
"XNAS"                  ->  Mic    ( 4 chars, registered in ISO 10383 snapshot)
"not-an-identifier"     ->  None
```

### Governing standards

Detection relies on the grammars and check digits of ISO 6166 (ISIN),
ISO 17442 (LEI), ISO 9362 (BIC), ISO 10383 (MIC), ANSI X9.6 (CUSIP),
ANSI X9.145 (FIGI), and the LSE SEDOL scheme.

---

## Standards index

| Identifier | Module | Governing standard |
|---|---|---|
| ISIN | `src/isin.rs` | ISO 6166 — International securities identification number |
| CUSIP | `src/cusip.rs` | ANSI X9.6 — CUSIP Numbering System (CUSIP Global Services) |
| CINS | `src/cusip.rs` | CUSIP International Numbering System (ANSI X9.6 scheme) |
| SEDOL | `src/sedol.rs` | London Stock Exchange — SEDOL Masterfile |
| LEI | `src/lei.rs` | ISO 17442; check digits per ISO/IEC 7064 MOD 97-10 |
| BIC | `src/bic.rs` | ISO 9362 — Business identifier code |
| MIC | `src/mic.rs` | ISO 10383 — Codes for exchanges and market identification |
| FIGI | `src/figi.rs` | ANSI X9.145 — Financial Instrument Global Identifier (OpenFIGI / OMG) |
| CFI | `src/cfi.rs` | ISO 10962 — Classification of financial instruments |
| WKN | `src/wkn.rs` | WM Datenservice — Wertpapierkennnummer |
| VALOR | `src/valor.rs` | SIX Financial Information — Valorennummer |
| Check-digit core | `src/checkdigit.rs` | ISO 6166, ISO/IEC 7064, ANSI X9.6, ANSI X9.145, LSE SEDOL |
| Country codes | `src/country.rs` | ISO 3166-1 alpha-2, plus registered ISIN substitute prefixes |
| Conversions | `src/convert.rs` | ISO 6166, ANSI X9.6, LSE SEDOL, SIX VALOR |
| Auto-detection | `src/detect.rs` | all of the above |

---

*Part of [Regit OS](https://www.regit.io) — the operating system for
investment products. From Luxembourg.*
