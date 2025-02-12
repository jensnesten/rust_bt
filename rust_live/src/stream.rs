use dotenv::dotenv;
use std::env;
use tokio_tungstenite::connect_async;
use tungstenite::Message;
use futures_util::StreamExt;
use reqwest::Client;
use chrono::Utc;
use rust_core::data_handler::parse_live_data;
use rust_core::engine::LiveData;

pub async fn single() -> Vec<LiveData> {
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
    
    // Send the subscription request via HTTP POST
    let subscription_payload = serde_json::json!({
        "ContextId": context_id,
        "RefreshRate": 1000,
        "ReferenceId": "price",
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": 4913
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
        .expect("Failed to send subscription request");
    //println!("Subscription response: {:?}", response.text().await.unwrap());
   
    let mut results: Vec<LiveData> = Vec::new();

    // Process incoming WebSocket messages and output the JSON response as-is.
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // If the message is text, print it directly.
                let live_data = parse_live_data(&text);
                results.push(live_data.clone());
                //println!("Received JSON: {:?}", live_data);
            }
            Ok(Message::Binary(bin)) => {
                // Convert binary data to a UTF-8 string.
                let text = String::from_utf8_lossy(&bin);
                let live_data = parse_live_data(&text);
                results.push(live_data.clone());
                //println!("Received JSON: {:?}", live_data);
            }
            Ok(other) => {
                println!("Received non-text message: {:?}", other);
            }
            Err(e) => {
                println!("WebSocket error: {:?}", e);
            }
        }
    }
    results
} 


pub async fn pairs() {

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
        "RefreshRate": 1000,
        "ReferenceId": "US500",
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": 4913
        }
    });

    let subscription_payload_2 = serde_json::json!({
        "ContextId": context_id,
        "RefreshRate": 1000,
        "ReferenceId": "DJIA",
        "Arguments": {
            "ClientKey": client_key,
            "AccountKey": account_key,
            "AssetType": "CfdOnIndex",
            "Uic": 4911 // Different Uic for the second instrument
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
                println!("Received JSON: {}", text);
            }
            Ok(Message::Binary(bin)) => {
                let text = String::from_utf8_lossy(&bin);
                println!("Received JSON: {}", text);
            }
            Ok(other) => {
                println!("Received non-text message: {:?}", other);
            }
            Err(e) => {
                println!("WebSocket error: {:?}", e);
            }
        }
    }
}