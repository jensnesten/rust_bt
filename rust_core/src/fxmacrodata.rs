#[derive(Clone, Debug)]
pub struct FXMacroDataClient {
    base_url: String,
    api_key: Option<String>,
}

impl FXMacroDataClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        Self {
            base_url: "https://api.fxmacrodata.com/v1".to_string(),
            api_key: if api_key.is_empty() { None } else { Some(api_key) },
        }
    }

    pub fn data_catalogue(&self, currency: &str) -> String { self.url(&format!("/data_catalogue/{}", norm(currency))) }
    pub fn announcements(&self, currency: &str, indicator: &str) -> String { self.url(&format!("/announcements/{}/{}", norm(currency), indicator)) }
    pub fn calendar(&self, currency: &str) -> String { self.url(&format!("/calendar/{}", norm(currency))) }
    pub fn predictions(&self, currency: &str, indicator: &str) -> String { self.url(&format!("/predictions/{}/{}", norm(currency), indicator)) }
    pub fn forex(&self, base: &str, quote: &str) -> String { self.url(&format!("/forex/{}/{}", norm(base), norm(quote))) }
    pub fn cot(&self, currency: &str) -> String { self.url(&format!("/cot/{}", norm(currency))) }
    pub fn commodities_latest(&self) -> String { self.url("/commodities/latest") }
    pub fn commodity(&self, indicator: &str) -> String { self.url(&format!("/commodities/{}", indicator)) }
    pub fn curves(&self, currency: &str) -> String { self.url(&format!("/curves/{}", norm(currency))) }
    pub fn curve_proxies(&self, currency: &str) -> String { self.url(&format!("/curve_proxies/{}", norm(currency))) }
    pub fn forward_curves(&self, currency: &str) -> String { self.url(&format!("/forward_curves/{}", norm(currency))) }
    pub fn market_sessions(&self) -> String { self.url("/market_sessions") }
    pub fn risk_sentiment(&self) -> String { self.url("/risk_sentiment") }
    pub fn news(&self, currency: &str) -> String { self.url(&format!("/news/{}", norm(currency))) }
    pub fn press_releases(&self, currency: &str) -> String { self.url(&format!("/press-releases/{}", norm(currency))) }
    pub fn central_bankers(&self, currency: &str) -> String { self.url(&format!("/central_bankers/{}", norm(currency))) }

    fn url(&self, path: &str) -> String {
        match &self.api_key {
            Some(key) if !key.is_empty() => format!("{}{}?api_key={}", self.base_url, path, encode(key)),
            _ => format!("{}{}", self.base_url, path),
        }
    }
}

fn norm(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn encode(value: &str) -> String {
    value.bytes().fold(String::new(), |mut output, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
        output
    })
}
