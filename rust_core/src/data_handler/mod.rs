use csv::ReaderBuilder;
use std::error::Error;
use crate::engine::OhlcData;
use crate::live_engine::LiveData;
use crate::live_engine::TickSnapshot;
use std::collections::HashMap;
use serde_json::Value;
use regex::Regex;
use nom;

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

//ACTUALLY WORKS

pub fn parse_live_data_with_reference_nom(raw: &str, expected_ref: &str) -> LiveData {
    let mut ticks: Vec<TickSnapshot> = Vec::new();
    let mut current: HashMap<String, TickSnapshot> = HashMap::new();

    // Look for the first occurrence of '{"'
    let json_start = match raw.find("{\"") {
        Some(idx) => idx,
        None => raw.find("{").unwrap_or(raw.len()),
    };

    // The prefix is everything before the JSON block.
    let prefix = &raw[..json_start];

    // Use expected_ref if found; else fallback to an alphanumeric token via nom.
    let inst = if prefix.contains(expected_ref) {
        expected_ref.to_string()
    } else {
        match nom::character::complete::alphanumeric1::<&str, nom::error::Error<&str>>(prefix) {
            Ok((_, s)) => s.to_string(),
            Err(_) => String::new(),
        }
    };

    // Locate the JSON block: from json_start to the last '}'.
    let json_str = if let Some(end) = raw.rfind("}") {
        &raw[json_start..=end]
    } else {
        ""
    };

    if !json_str.is_empty() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(quote) = parsed.get("Quote") {
                // Prefer instrument from JSON if available.
                let instrument = if inst.is_empty() {
                    parsed.get("ReferenceId")
                          .and_then(|v| v.as_str())
                          .unwrap_or("")
                          .to_string()
                } else {
                    inst
                };

                let date = parsed.get("LastUpdated")
                                 .and_then(|v| v.as_str())
                                 .unwrap_or("")
                                 .to_string();

                // Try to get Ask and Bid, fallback to Mid.
                let (ask_val, bid_val) = if let (Some(a), Some(b)) = (
                    quote.get("Ask").and_then(|v| v.as_f64()),
                    quote.get("Bid").and_then(|v| v.as_f64()),
                ) {
                    (a, b)
                } else if let Some(mid_val) = quote.get("Mid").and_then(|v| v.as_f64()) {
                    (mid_val, mid_val)
                } else {
                    (0.0, 0.0)
                };

                if ask_val != 0.0 || bid_val != 0.0 {
                    let tick_snapshot = TickSnapshot {
                        instrument: instrument.clone(),
                        date,
                        ask: ask_val,
                        bid: bid_val,
                    };

                    ticks.push(tick_snapshot.clone());
                    current.insert(instrument, tick_snapshot);
                }
            }
        }
    }

    LiveData { ticks, current }
}


pub fn parse_live_data_with_reference_nom2(
    raw: &str,
    expected_ref1: &str,
    expected_ref2: &str,
) -> LiveData {
    let mut ticks: Vec<TickSnapshot> = Vec::new();
    let mut current: HashMap<String, TickSnapshot> = HashMap::new();

    // Look for the first occurrence of '{"' or '{'
    let json_start = match raw.find("{\"") {
        Some(idx) => idx,
        None => raw.find("{").unwrap_or(raw.len()),
    };

    // The prefix is everything before the JSON block.
    let prefix = &raw[..json_start];

    // Check the prefix for expected_ref1 and expected_ref2.
    let inst = if prefix.contains(expected_ref1) {
        expected_ref1.to_string()
    } else if prefix.contains(expected_ref2) {
        expected_ref2.to_string()
    } else {
        // Fallback: extract the first alphanumeric token using nom.
        match nom::character::complete::alphanumeric1::<&str, nom::error::Error<&str>>(prefix) {
            Ok((_, s)) => s.to_string(),
            Err(_) => String::new(),
        }
    };

    // Locate the JSON block from json_start to the last '}'.
    let json_str = if let Some(end) = raw.rfind("}") {
        &raw[json_start..=end]
    } else {
        ""
    };

    if !json_str.is_empty() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(quote) = parsed.get("Quote") {
                // Prefer the instrument from JSON if available.
                let instrument = if inst.is_empty() {
                    parsed.get("ReferenceId")
                          .and_then(|v| v.as_str())
                          .unwrap_or("")
                          .to_string()
                } else {
                    inst
                };

                let date = parsed.get("LastUpdated")
                                 .and_then(|v| v.as_str())
                                 .unwrap_or("")
                                 .to_string();

                let (ask_val, bid_val) = if let (Some(a), Some(b)) = (
                    quote.get("Ask").and_then(|v| v.as_f64()),
                    quote.get("Bid").and_then(|v| v.as_f64()),
                ) {
                    (a, b)
                } else if let Some(mid_val) = quote.get("Mid").and_then(|v| v.as_f64()) {
                    (mid_val, mid_val)
                } else {
                    (0.0, 0.0)
                };

                if ask_val != 0.0 || bid_val != 0.0 {
                    let tick_snapshot = TickSnapshot {
                        instrument: instrument.clone(),
                        date,
                        ask: ask_val,
                        bid: bid_val,
                    };

                    ticks.push(tick_snapshot.clone());
                    current.insert(instrument, tick_snapshot);
                }
            }
        }
    }

    LiveData { ticks, current }
}
