//! Geolocation context, multi-currency negotiation, and European VAT calculation engine.
//!
//! Automatically infers client country, currency, timezone, and tax rules
//! from edge proxy headers (`CF-IPCountry`, `X-Country-Code`, `Accept-Language`).
//!
//! VAT rates are stored as integers (hundredths of a percent) for all 27 EU member
//! states, and [`VatRule::calculate_tax`] uses exact `i128` arithmetic — no `f64`
//! rounding drift. [`VatRule::is_supported`] distinguishes a configured (possibly
//! zero) rate from an unknown country.

use crate::billing::Money;
use axum::http::HeaderMap;

/// Contextual geolocation, currency, and tax parameters for an incoming client request.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeoIpContext {
    /// ISO 3166-1 alpha-2 country code (e.g. `"FR"`, `"US"`, `"DE"`, `"GB"`).
    pub country_code: String,
    /// Default currency code (ISO 4217, e.g. `"EUR"`, `"USD"`, `"GBP"`).
    pub currency_code: String,
    /// Inferred primary language tag (e.g. `"fr"`, `"en"`).
    pub language: String,
    /// Inferred IANA time zone (e.g. `"Europe/Paris"`).
    pub timezone: String,
    /// Whether the resolved country is a member of the European Union.
    pub is_eu: bool,
    /// Standard VAT/tax rate percentage (e.g. 20.0 for France).
    pub vat_rate_percent: f64,
}

impl Default for GeoIpContext {
    fn default() -> Self {
        Self {
            country_code: "US".to_string(),
            currency_code: "USD".to_string(),
            language: "en".to_string(),
            timezone: "America/New_York".to_string(),
            is_eu: false,
            vat_rate_percent: 0.0,
        }
    }
}

/// VAT and Sales Tax calculation results.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TaxBreakdown {
    /// Net amount before taxes.
    pub net: Money,
    /// Applied tax percentage (display only; the computation itself is exact integer math).
    pub tax_rate_percent: f64,
    /// Computed tax portion in minor units (cents).
    pub tax_amount: Money,
    /// Total gross amount (net + tax).
    pub gross: Money,
}

/// Standard VAT rate and regional rules registry.
pub struct VatRule;

impl VatRule {
    /// Standard VAT rate in hundredths of a percent (e.g. `25.5 %` → `2550`),
    /// or `None` when no rule is configured for the country.
    ///
    /// An integer basis avoids floating-point drift in tax computation and lets
    /// [`VatRule::is_supported`] distinguish "no rule" from a genuine zero rate.
    fn rate_hundredths(country: &str) -> Option<u32> {
        Some(match country.to_uppercase().as_str() {
            // ── 27 EU member states ──────────────────────────────────────────
            "AT" => 2000,
            "BE" => 2100,
            "BG" => 2000,
            "HR" => 2500,
            "CY" => 1900,
            "CZ" => 2100,
            "DK" => 2500,
            "EE" => 2200,
            "FI" => 2550,
            "FR" => 2000,
            "DE" => 1900,
            "GR" => 2400,
            "HU" => 2700,
            "IE" => 2300,
            "IT" => 2200,
            "LV" => 2100,
            "LT" => 2100,
            "LU" => 1700,
            "MT" => 1800,
            "NL" => 2100,
            "PL" => 2300,
            "PT" => 2300,
            "RO" => 2100,
            "SK" => 2300,
            "SI" => 2200,
            "ES" => 2100,
            "SE" => 2500,
            // ── Supported non-EU countries ──────────────────────────────────
            "GB" => 2000,
            "CH" => 810,
            "NO" => 2500,
            "JP" => 1000,
            "CA" => 500, // Federal GST
            "AU" => 1000,
            "NZ" => 1500,
            _ => return None,
        })
    }

    /// Get standard VAT/tax percentage for an ISO country code.
    ///
    /// Returns `0.0` for an unsupported country. Use [`VatRule::is_supported`] to
    /// distinguish "no rule configured" from a real zero rate.
    pub fn rate_for(country: &str) -> f64 {
        Self::rate_hundredths(country)
            .map(|h| h as f64 / 100.0)
            .unwrap_or(0.0)
    }

    /// Whether a VAT rule is configured for `country`.
    ///
    /// [`VatRule::rate_for`] returns `0.0` both for a genuine 0 % rate and for an
    /// unknown country; this predicate makes the distinction unambiguous.
    pub fn is_supported(country: &str) -> bool {
        Self::rate_hundredths(country).is_some()
    }

    /// Check if country belongs to the European Union (EU VAT area).
    pub fn is_eu(country: &str) -> bool {
        matches!(
            country.to_uppercase().as_str(),
            "AT" | "BE"
                | "BG"
                | "HR"
                | "CY"
                | "CZ"
                | "DK"
                | "EE"
                | "FI"
                | "FR"
                | "DE"
                | "GR"
                | "HU"
                | "IE"
                | "IT"
                | "LV"
                | "LT"
                | "LU"
                | "MT"
                | "NL"
                | "PL"
                | "PT"
                | "RO"
                | "SK"
                | "SI"
                | "ES"
                | "SE"
        )
    }

    /// Suggest default ISO currency for a country.
    pub fn currency_for(country: &str) -> &'static str {
        match country.to_uppercase().as_str() {
            "US" => "USD",
            "GB" => "GBP",
            "JP" => "JPY",
            "CH" => "CHF",
            "CA" => "CAD",
            "AU" => "AUD",
            c if Self::is_eu(c) => "EUR",
            _ => "USD",
        }
    }

    /// Compute an exact, penny-rounded tax breakdown from a net [`Money`] amount.
    ///
    /// Only integer arithmetic (`i128`) is used for the money computation: the rate
    /// is expressed in hundredths of a percent, so
    /// `tax = round(net_minor * rate_hundredths / 10_000)`, rounded half away from
    /// zero to the nearest cent. No `f64` is involved, so there is no rounding drift.
    /// Unsupported countries produce a `0` tax (see [`VatRule::is_supported`]).
    pub fn calculate_tax(net: Money, country: &str) -> TaxBreakdown {
        let rate_hundredths = Self::rate_hundredths(country).unwrap_or(0) as i128;
        let curr = net.currency.clone();
        let denominator = 10_000i128;
        let numerator = net.amount_minor * rate_hundredths;
        let tax_minor = if numerator >= 0 {
            (numerator + denominator / 2) / denominator
        } else {
            -(((-numerator) + denominator / 2) / denominator)
        };

        let tax_amount = Money {
            amount_minor: tax_minor,
            currency: curr.clone(),
        };
        let gross = Money {
            amount_minor: net.amount_minor + tax_minor,
            currency: curr,
        };

        TaxBreakdown {
            net,
            tax_rate_percent: rate_hundredths as f64 / 100.0,
            tax_amount,
            gross,
        }
    }
}

/// Extract geolocation context from HTTP request headers.
pub fn extract_geo_context(headers: &HeaderMap) -> GeoIpContext {
    // 1. Resolve country from Cloudflare / Edge proxy headers
    let country_code = headers
        .get("cf-ipcountry")
        .or_else(|| headers.get("x-country-code"))
        .or_else(|| headers.get("x-geo-country"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_uppercase())
        .filter(|s| s.len() == 2)
        .unwrap_or_else(|| "US".to_string());

    // 2. Resolve language from Accept-Language (e.g. "fr-FR,fr;q=0.9,en;q=0.8")
    let language = headers
        .get("accept-language")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.split(';').next().unwrap_or(s).trim())
        .map(|s| s.split('-').next().unwrap_or(s).to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "en".to_string());

    // 3. Resolve timezone
    let timezone = headers
        .get("cf-timezone")
        .or_else(|| headers.get("x-timezone"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| match country_code.as_str() {
            "FR" => "Europe/Paris".to_string(),
            "DE" => "Europe/Berlin".to_string(),
            "GB" => "Europe/London".to_string(),
            "JP" => "Asia/Tokyo".to_string(),
            _ => "UTC".to_string(),
        });

    let is_eu = VatRule::is_eu(&country_code);
    let currency_code = VatRule::currency_for(&country_code).to_string();
    let vat_rate_percent = VatRule::rate_for(&country_code);

    GeoIpContext {
        country_code,
        currency_code,
        language,
        timezone,
        is_eu,
        vat_rate_percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_geo_context_from_cloudflare_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-ipcountry", HeaderValue::from_static("FR"));
        headers.insert(
            "accept-language",
            HeaderValue::from_static("fr-FR,fr;q=0.9,en;q=0.8"),
        );
        headers.insert("cf-timezone", HeaderValue::from_static("Europe/Paris"));

        let ctx = extract_geo_context(&headers);
        assert_eq!(ctx.country_code, "FR");
        assert_eq!(ctx.currency_code, "EUR");
        assert_eq!(ctx.language, "fr");
        assert_eq!(ctx.timezone, "Europe/Paris");
        assert!(ctx.is_eu);
        assert_eq!(ctx.vat_rate_percent, 20.0);
    }

    #[test]
    fn test_vat_tax_calculation() {
        let net = Money::eur(10000); // 100.00 EUR
        let breakdown = VatRule::calculate_tax(net, "FR");

        assert_eq!(breakdown.tax_rate_percent, 20.0);
        assert_eq!(breakdown.tax_amount.amount_minor, 2000); // 20.00 EUR
        assert_eq!(breakdown.gross.amount_minor, 12000); // 120.00 EUR
    }

    #[test]
    fn test_all_27_eu_member_vat_rates() {
        // (ISO code, standard VAT rate in %) for all 27 EU member states.
        let eu: &[(&str, f64)] = &[
            ("AT", 20.0),
            ("BE", 21.0),
            ("BG", 20.0),
            ("HR", 25.0),
            ("CY", 19.0),
            ("CZ", 21.0),
            ("DK", 25.0),
            ("EE", 22.0),
            ("FI", 25.5),
            ("FR", 20.0),
            ("DE", 19.0),
            ("GR", 24.0),
            ("HU", 27.0),
            ("IE", 23.0),
            ("IT", 22.0),
            ("LV", 21.0),
            ("LT", 21.0),
            ("LU", 17.0),
            ("MT", 18.0),
            ("NL", 21.0),
            ("PL", 23.0),
            ("PT", 23.0),
            ("RO", 21.0),
            ("SK", 23.0),
            ("SI", 22.0),
            ("ES", 21.0),
            ("SE", 25.0),
        ];
        assert_eq!(eu.len(), 27);
        for (code, expected) in eu {
            assert!(VatRule::is_eu(code), "{code} must be an EU member");
            assert!(
                VatRule::is_supported(code),
                "{code} must have a configured VAT rule"
            );
            assert_eq!(VatRule::rate_for(code), *expected, "rate for {code}");
            assert_eq!(VatRule::currency_for(code), "EUR");
            // Input is case-insensitive.
            assert_eq!(VatRule::rate_for(&code.to_lowercase()), *expected);
        }
    }

    #[test]
    fn test_is_supported_distinguishes_unknown_country() {
        assert!(VatRule::is_supported("FR"));
        assert!(VatRule::is_supported("CH")); // non-EU but supported
        assert!(!VatRule::is_supported("XX"));
        assert!(!VatRule::is_supported(""));
        // rate_for() alone is ambiguous (0.0 could be a real 0 % or "unsupported").
        assert_eq!(VatRule::rate_for("XX"), 0.0);
        assert!(!VatRule::is_supported("XX"));
    }

    #[test]
    fn test_calculate_tax_exact_penny_rounding() {
        // 999 cents at 20 % -> 199.8 -> 200 cents (rounded to the nearest cent).
        let fr = VatRule::calculate_tax(Money::eur(999), "FR");
        assert_eq!(fr.tax_rate_percent, 20.0);
        assert_eq!(fr.tax_amount.amount_minor, 200);
        assert_eq!(fr.gross.amount_minor, 1199);

        // 999 cents at 25.5 % (FI) -> 254.745 -> 255 cents, exactly (no f64 drift).
        let fi = VatRule::calculate_tax(Money::eur(999), "FI");
        assert_eq!(fi.tax_rate_percent, 25.5);
        assert_eq!(fi.tax_amount.amount_minor, 255);
        assert_eq!(fi.gross.amount_minor, 1254);

        // Large amount: 1,000,000.00 EUR at 19 % -> 190,000.00 EUR exactly.
        let big = VatRule::calculate_tax(Money::eur(100_000_000), "DE");
        assert_eq!(big.tax_amount.amount_minor, 19_000_000);
        assert_eq!(big.gross.amount_minor, 119_000_000);

        // Negative net (refund): rounding is half away from zero.
        let refund = VatRule::calculate_tax(Money::eur(-999), "FR");
        assert_eq!(refund.tax_amount.amount_minor, -200);

        // Unsupported country: no invented tax.
        let unknown = VatRule::calculate_tax(Money::eur(10000), "XX");
        assert_eq!(unknown.tax_rate_percent, 0.0);
        assert_eq!(unknown.tax_amount.amount_minor, 0);
        assert_eq!(unknown.gross.amount_minor, 10000);
    }
}
