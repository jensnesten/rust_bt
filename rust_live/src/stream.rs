use dotenv::dotenv;
use std::env;
use tokio_tungstenite::connect_async;
use tungstenite::Message;
use futures_util::StreamExt;
use reqwest::Client;
use chrono::Utc;
use rust_core::data_handler::{parse_live_data_with_reference_nom2, parse_live_data_with_reference_nom};
use rust_core::live_engine::LiveData;
use tokio::sync::mpsc::UnboundedSender;
use regex::Regex;


fn clean_raw_text(raw: &str, ref_ids: &[&str]) -> String {
    // Remove all null characters.
    let cleaned = raw.replace("\0", "");
    
    // Find the beginning of the JSON block.
    if let Some(json_start) = cleaned.find("{\"") {
        let prefix = &cleaned[..json_start];
        let json_part = &cleaned[json_start..];

        // Build a new prefix that only contains the reference IDs (if present).
        let mut kept = String::new();
        for ref_id in ref_ids {
            if prefix.contains(ref_id) {
                if !kept.is_empty() {
                    kept.push(' ');
                }
                kept.push_str(ref_id);
            }
        }
        // Optionally, trim any extra whitespace.
        return format!("{}{}", kept.trim(), json_part);
    }
    // If no JSON block is found, return the cleaned string.
    cleaned
}


fn split_messages(raw: &str, ref_ids: &[&str]) -> Vec<String> {
    // Remove all null characters.
    let cleaned = raw.replace("\0", "");
    
    // Build a regex pattern that matches any one of the reference IDs followed by '{'
    // For example, if ref_ids are ["DJIA", "US500"], pattern becomes: (DJIA|US500)\{
    let pattern = format!("({})\\{{", ref_ids.join("|"));
    let re = Regex::new(&pattern).unwrap();
    
    // Collect the start positions for each new message.
    let mut indices = Vec::new();
    for mat in re.find_iter(&cleaned) {
        indices.push(mat.start());
    }
    
    // If no reference id boundary is found, return the whole cleaned string.
    if indices.is_empty() {
        return vec![cleaned];
    }
    
    // Now split the cleaned string at these indices.
    let mut segments = Vec::new();
    // Ensure we include the very beginning if needed.
    if indices[0] != 0 {
        segments.push(cleaned[0..indices[0]].trim().to_string());
    }
    for i in 0..indices.len() {
        let start = indices[i];
        let end = if i + 1 < indices.len() { indices[i+1] } else { cleaned.len() };
        let seg = cleaned[start..end].trim().to_string();
        if !seg.is_empty() {
            segments.push(seg);
        }
    }
    segments
}



// continuously streams live data and sends parsed messages over the channel
pub async fn stream_live_data(tx: UnboundedSender<LiveData>, reference_id: &str, uic: i32) {
    dotenv().ok();

    // load api credentials from .env
    let access_token = env::var("ACCESS_TOKEN").expect("missing ACCESS_TOKEN in .env");
    let account_key = env::var("ACCOUNT_KEY").expect("missing ACCOUNT_KEY in .env");
    let client_key = env::var("CLIENT_KEY").expect("missing CLIENT_KEY in .env");

    // build context id and streamer url
    let context_id = format!("MyApp42069{}", Utc::now().timestamp_millis());
    let streamer_url = format!(
        "wss://streaming.saxobank.com/sim/openapi/streamingws/connect?authorization=BEARER%20{}&contextId={}",
        access_token, context_id
    );
    println!("connecting to saxo bank websocket...");
    let (ws_stream, _) = connect_async(&streamer_url)
        .await
        .expect("failed to connect: ensure tls is enabled");
    println!("connected.");

    // split the websocket stream into write (unused) and read parts
    let (_write, mut read) = ws_stream.split();

    let reference_id = reference_id.to_string();

    // send the subscription request via HTTP POST
    let subscription_payload = serde_json::json!({
        "ContextId": context_id,
        "RefreshRate": 1000,
        "ReferenceId": reference_id,
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": uic
        }
    });
    let client = Client::new();
    let response = client
        .post("https://gateway.saxobank.com/sim/openapi/trade/v1/prices/subscriptions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&subscription_payload)
        .send()
        .await
        .expect("failed to send subscription request");
     println!("subscription response: {:?}", response.text().await.unwrap());

    // continuously process websocket messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
            
            }
            Ok(Message::Binary(bin)) => {
                let text = String::from_utf8_lossy(&bin);
                //println!("text: {:?}", text);
                let live_data = parse_live_data_with_reference_nom(&text, &reference_id);
                let _ = tx.send(live_data.clone());
                //println!("live data: {:?}", live_data);
            }
            Ok(other) => {
                println!("received non-text message: {:?}", other);
            }
            Err(e) => {
                println!("websocket error: {:?}", e);
            }
        }
    }
}


pub async fn pairs(tx: UnboundedSender<LiveData>,reference_id_1: &str, uic_1: i32, reference_id_2: &str, uic_2: i32) {

    dotenv().ok();

    // Load API credentials from .env
    let access_token = env::var("ACCESS_TOKEN").expect("Missing ACCESS_TOKEN in .env");
    let account_key = env::var("ACCOUNT_KEY").expect("Missing ACCOUNT_KEY in .env");
    let client_key = env::var("CLIENT_KEY").expect("Missing CLIENT_KEY in .env");

    // Build a context ID and streamer URL
    let context_id = format!("MyApp42069{}", Utc::now().timestamp_millis());
    let streamer_url = format!(
        "wss://streaming.saxobank.com/sim/openapi/streamingws/connect?authorization=BEARER%20{}&contextId={}",
        access_token, context_id
    );

    println!("Connecting to Saxo Bank WebSocket...");
    let (ws_stream, _) = connect_async(&streamer_url)
        .await
        .expect("Failed to connect: Ensure TLS is enabled in your dependencies (e.g., with native-tls or rustls-tls-webpki-roots)");
    println!("Connected.");

    // Split the WebSocket stream into write (unused) and read parts.
    let (_write, mut read) = ws_stream.split();

    // Create two subscription payloads with different Uic values and ReferenceIds.
    let subscription_payload_1 = serde_json::json!({
        "ContextId": context_id,
        "RefreshRate": 2000,
        "ReferenceId": reference_id_1,
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": uic_1
        }
    });

    let subscription_payload_2 = serde_json::json!({
        "ContextId": context_id,
        "RefreshRate": 2000,
        "ReferenceId": reference_id_2,
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": uic_2
        }
    });

    let client = Client::new();

    // Send the first subscription request
    let response1 = client
        .post("https://gateway.saxobank.com/sim/openapi/trade/v1/prices/subscriptions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&subscription_payload_1)
        .send()
        .await
        .expect("Failed to send subscription request for instrument 1");
    println!("Subscription response 1: {:?}", response1.text().await.unwrap());

    // Send the second subscription request
    let response2 = client
        .post("https://gateway.saxobank.com/sim/openapi/trade/v1/prices/subscriptions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&subscription_payload_2)
        .send()
        .await
        .expect("Failed to send subscription request for instrument 2");
    println!("Subscription response 2: {:?}", response2.text().await.unwrap());

    // Process incoming WebSocket messages and output the JSON response as-is.
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                
            }
            Ok(Message::Binary(bin)) => {
                let text = String::from_utf8_lossy(&bin);
                let clean_text = clean_raw_text(&text, &[ "DJIA", "US500" ]);
                println!("text: {:?}", clean_text);
                let segments = split_messages(&clean_text, &[ "DJIA", "US500" ]);
                for segment in segments {
                    println!("Segment: {:?}", segment);
                    // Now pass each segment to your parser:
                    let live_data = parse_live_data_with_reference_nom2(&segment, "DJIA", "US500");
                    // Process or send live_data as needed...
                    let _ = tx.send(live_data);
                }
            }
            Ok(other) => {
                println!("received non-text message: {:?}", other);
            }
            Err(e) => {
                println!("websocket error: {:?}", e);
            }
        }
    }
}

pub async fn stream_live_data_pairs(tx: UnboundedSender<LiveData>, reference_id_1: &str, uic_1: i32, reference_id_2: &str, uic_2: i32) {
    dotenv().ok();

    // load api credentials from .env
    let access_token = env::var("ACCESS_TOKEN").expect("Missing ACCESS_TOKEN in .env");
    let account_key = env::var("ACCOUNT_KEY").expect("Missing ACCOUNT_KEY in .env");
    let client_key = env::var("CLIENT_KEY").expect("Missing CLIENT_KEY in .env");

    // Build a context ID and streamer URL
    let context_id = format!("MyApp42069{}", Utc::now().timestamp_millis());
    let streamer_url = format!(
        "wss://streaming.saxobank.com/sim/openapi/streamingws/connect?authorization=BEARER%20{}&contextId={}",
        access_token, context_id
    );

    println!("Connecting to Saxo Bank WebSocket...");
    let (ws_stream, _) = connect_async(&streamer_url)
        .await
        .expect("Failed to connect: Ensure TLS is enabled in your dependencies (e.g., with native-tls or rustls-tls-webpki-roots)");
    println!("Connected.");

    // Split the WebSocket stream into write (unused) and read parts.
    let (_write, mut read) = ws_stream.split();

    // Create two subscription payloads with different Uic values and ReferenceIds.
    let subscription_payload_1 = serde_json::json!({
        "ContextId": context_id,
        "RefreshRate": 1000,
        "ReferenceId": reference_id_1,
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": uic_1
        }
    });

    let subscription_payload_2 = serde_json::json!({
        "ContextId": context_id,
        "RefreshRate": 1000,
        "ReferenceId": reference_id_2,
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": uic_2
        }
    });

    let client = Client::new();

    let response1 = client
        .post("https://gateway.saxobank.com/sim/openapi/trade/v1/prices/subscriptions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&subscription_payload_1)
        .send()
        .await
        .expect("Failed to send subscription request for instrument 1");
        println!("Subscription response 1: {:?}", response1.text().await.unwrap());

// Send the second subscription request
    let response2 = client
        .post("https://gateway.saxobank.com/sim/openapi/trade/v1/prices/subscriptions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&subscription_payload_2)
        .send()
        .await
        .expect("Failed to send subscription request for instrument 2");
        println!("Subscription response 2: {:?}", response2.text().await.unwrap());

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                
            }
            Ok(Message::Binary(bin)) => {
                let text = String::from_utf8_lossy(&bin);
                let live_data_vec = parse_live_data_with_reference_nom2(&text, &reference_id_1, &reference_id_2);
                
            }
            Ok(other) => {
                println!("received non-text message: {:?}", other);
            }
            Err(e) => {
                println!("websocket error: {:?}", e);
            }
        }
    }
}

