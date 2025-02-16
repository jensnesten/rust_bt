use csv::ReaderBuilder;
use std::error::Error;
use crate::engine::OhlcData;
use crate::live_engine::LiveData;
use serde_json::Value;
use regex::Regex;

// data handler for simple csv
pub fn handle_ohlc(path: &str) -> Result<OhlcData, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)?;

    let mut date = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut close2 = Vec::new();
    
    for result in rdr.records() {
        let record = result?;
        date.push(record[0].to_string());
        open.push(record[1].parse::<f64>()?);
        high.push(record[2].parse::<f64>()?);
        low.push(record[3].parse::<f64>()?);
        close.push(record[4].parse::<f64>()?);
        let close2_val = if record[5].trim().is_empty() {
            0.0
        } else {
            record[5].parse::<f64>()?
        };
        close2.push(close2_val);
    }
    
    Ok(OhlcData {
        date,
        open,
        high,
        low,
        close,
        close2,
        volume: None,
    })
}

pub fn parse_live_data(raw: &str) -> LiveData {
    // create a live data instance with vector fields
    let mut live_data = LiveData {
        instrument: Vec::new(),
        date: Vec::new(),
        ask: Vec::new(),
        bid: Vec::new(),
    };

    // find the first occurrence of '{'
    if let Some(idx) = raw.find('{') {
        // extract raw instrument string from before the json block
        let raw_instrument = raw[..idx].trim_matches(|c: char| !c.is_alphanumeric());
        // regex to capture letters followed by digits, ignoring any trailing extra characters
        let re = Regex::new(r"^([A-Za-z]+(?:[0-9]+)?)").unwrap();
        let instrument = if let Some(cap) = re.captures(raw_instrument) {
            cap.get(1).unwrap().as_str().to_string()
        } else {
            raw_instrument.to_string()
        };

        // get the json part
        let json_str = &raw[idx..];

        if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
            if let Some(quote) = parsed.get("Quote") {
                if let (Some(ask_val), Some(bid_val)) = (
                    quote.get("Ask").and_then(|v| v.as_f64()),
                    quote.get("Bid").and_then(|v| v.as_f64()),
                ) {
                    live_data.instrument.push(instrument);
                    live_data.date.push(
                        parsed
                            .get("LastUpdated")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );
                    live_data.ask.push(ask_val);
                    live_data.bid.push(bid_val);
                }
            }
        }
    }
    live_data
}

