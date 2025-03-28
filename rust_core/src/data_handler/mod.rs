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

/// Parse potentially concatenated streaming data with multiple instruments
pub fn parse_multipart_live_data(raw: &str) -> LiveData {
    let mut ticks: Vec<TickSnapshot> = Vec::new();
    let mut current: HashMap<String, TickSnapshot> = HashMap::new();

    // Convert to bytes for safer manipulation
    let raw_bytes = raw.as_bytes();
    
    // Instrument identifiers as byte patterns instead of strings
    let us500_pattern = b"US500";
    let djia_pattern = b"DJIA";
    
    // Find JSON objects - more resilient approach
    let mut start_index = 0;
    while start_index < raw_bytes.len() {
        // Look for instrument identifiers
        let mut instrument = String::new();
        
        // Check for US500
        if start_index + us500_pattern.len() <= raw_bytes.len() &&
           &raw_bytes[start_index..start_index + us500_pattern.len()] == us500_pattern {
            instrument = "US500".to_string();
        }
        // Check for DJIA
        else if start_index + djia_pattern.len() <= raw_bytes.len() &&
                &raw_bytes[start_index..start_index + djia_pattern.len()] == djia_pattern {
            instrument = "DJIA".to_string();
        }
        
        // Skip if no instrument found
        if instrument.is_empty() {
            start_index += 1;
            continue;
        }
        
        // Find JSON start
        let mut json_start = start_index;
        while json_start < raw_bytes.len() {
            if raw_bytes[json_start] == b'{' {
                break;
            }
            json_start += 1;
        }
        
        if json_start >= raw_bytes.len() {
            start_index += 1;
            continue;
        }
        
        // Find JSON end (matching closing brace)
        let mut json_end = json_start + 1; 
        let mut brace_count = 1;
        
        while json_end < raw_bytes.len() && brace_count > 0 {
            if raw_bytes[json_end] == b'{' {
                brace_count += 1;
            } else if raw_bytes[json_end] == b'}' {
                brace_count -= 1;
            }
            json_end += 1;
        }
        
        // Extract JSON if we found a complete object
        if brace_count == 0 {
            // Safely convert bytes to string, replacing invalid UTF-8
            let json_str = String::from_utf8_lossy(&raw_bytes[json_start..json_end]).to_string();
            
            // Parse JSON
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(quote) = parsed.get("Quote") {
                    let date = parsed.get("LastUpdated")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    // Extract bid/ask prices
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
                    
                    // Only process valid price data
                    if ask_val > 0.0 || bid_val > 0.0 {
                        let tick_snapshot = TickSnapshot {
                            instrument: instrument.clone(),
                            date,
                            ask: ask_val,
                            bid: bid_val,
                        };
                        
                        ticks.push(tick_snapshot.clone());
                        current.insert(instrument.clone(), tick_snapshot);
                            
                        // Debug output
                        println!("{}: ask: {}, bid: {}", instrument, ask_val, bid_val);
                    }
                }
            }
            
            // Move past this JSON object
            start_index = json_end;
        } else {
            // If we couldn't find a complete JSON object, move forward
            start_index += 1;
        }
    }
    
    LiveData { ticks, current }
}
