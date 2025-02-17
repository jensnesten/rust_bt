use csv::ReaderBuilder;
use std::error::Error;
use crate::engine::OhlcData;
use crate::live_engine::LiveData;
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

pub fn parse_live_data(raw: &str) -> LiveData {
    // create live data instance with vector fields
    let mut live_data = LiveData {
        instrument: Vec::new(),
        date: Vec::new(),
        ask: Vec::new(),
        bid: Vec::new(),
    };

    // find first '{'
    if let Some(idx) = raw.find('{') {
        // extract raw prefix from before the json block
        let raw_prefix = &raw[..idx];
        // remove control characters from the prefix
        let cleaned_prefix: String = raw_prefix.chars().filter(|c| !c.is_control()).collect();
        // use regex to get letters and digits from anywhere in the prefix
        let re = regex::Regex::new(r"[A-Za-z]+[0-9]*").unwrap();
        let extracted_instrument = if let Some(mat) = re.find(&cleaned_prefix) {
            mat.as_str().to_string()
        } else {
            String::new()
        };

        // get json part
        let json_str = &raw[idx..];

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            // always prefer the instrument from the json, if available
            let instrument = if let Some(ref_id) = parsed.get("ReferenceId").and_then(|v| v.as_str()) {
                ref_id.to_string()
            } else {
                extracted_instrument
            };

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

pub fn parse_live_data_with_reference(raw: &str, expected_ref: &str) -> LiveData {
    // create live data instance
    let mut live_data = LiveData {
        instrument: Vec::new(),
        date: Vec::new(),
        ask: Vec::new(),
        bid: Vec::new(),
    };

    // find the first occurrence of '{'
    if let Some(idx) = raw.find('{') {
        // extract the prefix (raw instrument part)
        let raw_prefix = &raw[..idx];

        // check if the expected reference string is present before the json part
        let instrument = if raw_prefix.contains(expected_ref) {
            expected_ref.to_string()
        } else {
            // fallback: clean the prefix by removing control characters
            let cleaned_prefix: String = raw_prefix.chars().filter(|c| !c.is_control()).collect();
            // use a regex to find a sequence of letters optionally followed by digits anywhere in the cleaned prefix
            let re = regex::Regex::new(r"[A-Za-z]+[0-9]*").unwrap();
            if let Some(mat) = re.find(&cleaned_prefix) {
                mat.as_str().to_string()
            } else {
                String::new()
            }
        };

        // extract the json part
        let json_str = &raw[idx..];

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(quote) = parsed.get("Quote") {
                if let (Some(ask_val), Some(bid_val)) = (
                    quote.get("Ask").and_then(|v| v.as_f64()),
                    quote.get("Bid").and_then(|v| v.as_f64()),
                ) {
                    // if our computed instrument ends up empty, we fallback to the ReferenceId from the json
                    let instrument = if instrument.is_empty() {
                        parsed.get("ReferenceId")
                              .and_then(|v| v.as_str())
                              .unwrap_or("")
                              .to_string()
                    } else {
                        instrument
                    };

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



pub fn parse_live_data_pairs(raw: &str, expected_refs: &[&str]) -> Vec<LiveData> {
    // create vec to accumulate live data
    let mut results = Vec::new();
    let mut pos = 0;
    let len = raw.len();

    // iterate over raw string to extract each json block using simple brace matching
    while pos < len {
        // find first '{' which starts the json
        if let Some(json_start_idx) = raw[pos..].find('{') {
            let json_start = pos + json_start_idx;
            let mut brace_count = 0;
            let mut in_string = false;
            let mut json_end = json_start;
            for (i, c) in raw[json_start..].char_indices() {
                if c == '"' {
                    in_string = !in_string; // toggle quoting
                }
                if !in_string {
                    if c == '{' {
                        brace_count += 1;
                    } else if c == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            json_end = json_start + i;
                            break;
                        }
                    }
                }
            }
            // extract json block
            let json_str = &raw[json_start..=json_end];

            // parse json
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                // use the json's ReferenceId field to determine the instrument
                if let Some(ref_id) = parsed.get("ReferenceId").and_then(|v| v.as_str()) {
                    // process only if the ref_id is one of our expected instruments
                    if expected_refs.contains(&ref_id) {
                        // check for prices in the Quote field
                        if let Some(quote) = parsed.get("Quote") {
                            if let (Some(ask_val), Some(bid_val)) = (
                                quote.get("Ask").and_then(|v| v.as_f64()),
                                quote.get("Bid").and_then(|v| v.as_f64()),
                            ) {
                                let mut live_data = LiveData {
                                    instrument: Vec::new(),
                                    date: Vec::new(),
                                    ask: Vec::new(),
                                    bid: Vec::new(),
                                };
                                live_data.instrument.push(ref_id.to_string());
                                live_data.date.push(
                                    parsed
                                        .get("LastUpdated")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                );
                                live_data.ask.push(ask_val);
                                live_data.bid.push(bid_val);
                                results.push(live_data);
                            }
                        }
                    }
                }
            }
            pos = json_end + 1; // move past this json block
        } else {
            break;
        }
    }
    results
}





//ACTUALLY WORKS

pub fn parse_live_data_with_reference_nom(raw: &str, expected_ref: &str) -> LiveData {
    // create live data instance
    let mut live_data = LiveData {
        instrument: Vec::new(),
        date: Vec::new(),
        ask: Vec::new(),
        bid: Vec::new(),
    };

    // look for the first occurrence of '{"'
    let json_start = match raw.find("{\"") {
        Some(idx) => idx,
        None => raw.find("{").unwrap_or(raw.len()),
    };

    // prefix is everything before the json block
    let prefix = &raw[..json_start];

    // use expected ref if present in prefix, else extract first alphanumeric using nom
    let inst = if prefix.contains(expected_ref) {
        expected_ref.to_string()
    } else {
        match nom::character::complete::alphanumeric1::<&str, nom::error::Error<&str>>(prefix) {
            Ok((_, s)) => s.to_string(),
            Err(_) => String::new(),
        }
    };

    // locate json block from json_start until the last '}'
    let json_str = if let Some(end) = raw.rfind("}") {
        &raw[json_start..=end]
    } else {
        ""
    };

    if !json_str.is_empty() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(quote) = parsed.get("Quote") {
                // try to read ask and bid; if missing, fallback to mid
                if let (Some(ask_val), Some(bid_val)) = (
                    quote.get("Ask").and_then(|v| v.as_f64()),
                    quote.get("Bid").and_then(|v| v.as_f64()),
                ) {
                    let instrument = if inst.is_empty() {
                        parsed
                            .get("ReferenceId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        inst
                    };

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
                } else if let Some(mid_val) = quote.get("Mid").and_then(|v| v.as_f64()) {
                    let instrument = if inst.is_empty() {
                        parsed
                            .get("ReferenceId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        inst
                    };

                    live_data.instrument.push(instrument);
                    live_data.date.push(
                        parsed
                            .get("LastUpdated")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );
                    live_data.ask.push(mid_val);
                    live_data.bid.push(mid_val);
                }
            }
        }
    }
    live_data
}


pub fn parse_live_data_with_reference_nom2(
    raw: &str,
    expected_ref1: &str,
    expected_ref2: &str,
) -> LiveData {
    // Create live data instance.
    let mut live_data = LiveData {
        instrument: Vec::new(),
        date: Vec::new(),
        ask: Vec::new(),
        bid: Vec::new(),
    };

    // Look for the first occurrence of '{"' or '{'
    let json_start = match raw.find("{\"") {
        Some(idx) => idx,
        None => raw.find("{").unwrap_or(raw.len()),
    };

    // The prefix is everything before the JSON block.
    let prefix = &raw[..json_start];

    // Check the prefix for the expected reference IDs.
    let inst = if prefix.contains(expected_ref1) {
        expected_ref1.to_string()
    } else if prefix.contains(expected_ref2) {
        expected_ref2.to_string()
    } else {
        // Fallback: try to extract the first alphanumeric token from the prefix using nom.
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
                // Try to read Ask and Bid; if missing, fallback to Mid.
                if let (Some(ask_val), Some(bid_val)) = (
                    quote.get("Ask").and_then(|v| v.as_f64()),
                    quote.get("Bid").and_then(|v| v.as_f64()),
                ) {
                    let instrument = if inst.is_empty() {
                        parsed
                            .get("ReferenceId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        inst
                    };

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
                } else if let Some(mid_val) = quote.get("Mid").and_then(|v| v.as_f64()) {
                    let instrument = if inst.is_empty() {
                        parsed
                            .get("ReferenceId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        inst
                    };

                    live_data.instrument.push(instrument);
                    live_data.date.push(
                        parsed
                            .get("LastUpdated")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );
                    live_data.ask.push(mid_val);
                    live_data.bid.push(mid_val);
                }
            }
        }
    }
    live_data
}