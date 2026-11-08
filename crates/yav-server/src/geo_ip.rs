//! Geolocation context, multi-currency negotiation, and European VAT calculation engine.
//!
//! Automatically infers client country, currency, timezone, and tax rules
//! from edge proxy headers (`CF-IPCountry`, `X-Country-Code`, `Accept-Language`).

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
    /// Applied tax percentage.
    pub tax_rate_percent: f64,
    /// Computed tax portion in minor units (cents).
    pub tax_amount: Money,
    /// Total gross amount (net + tax).
    pub gross: Money,
}

/// Standard VAT rate and regional rules registry.
pub struct VatRule;

impl VatRule {
    /// Get standard VAT/tax percentage for an ISO country code.
    pub fn rate_for(country: &str) -> f64 {
        match country.to_uppercase().as_str() {
            "FR" => 20.0,
            "DE" => 19.0,
            "ES" => 21.0,
            "IT" => 22.0,
            "NL" => 21.0,
            "BE" => 21.0,
            "IE" => 23.0,
            "PT" => 23.0,
            "AT" => 20.0,
            "PL" => 23.0,
            "SE" => 25.0,
            "DK" => 25.0,
            "FI" => 25.5,
            "GB" => 20.0,
            "CH" => 8.1,
            "NO" => 25.0,
            "JP" => 10.0,
            "CA" => 5.0, // Federal GST
            "AU" => 10.0,
            "NZ" => 15.0,
            _ => 0.0,
        }
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

    /// Compute exact penny-rounded tax breakdown from a net [`Money`] amount.
    pub fn calculate_tax(net: Money, country: &str) -> TaxBreakdown {
        let rate = Self::rate_for(country);
        let curr = net.currency.clone();
        // Tax = round(net * (rate / 100))
        let tax_minor = ((net.amount_minor as f64) * (rate / 100.0)).round() as i128;
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
            tax_rate_percent: rate,
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
}
