use std::sync::{Arc, Mutex};
use warp::Filter;
use futures::{StreamExt, SinkExt};
use tokio::time::{sleep, Duration};
use chrono::Utc;
use serde::Serialize;
use warp::cors::Cors;

#[derive(Clone, Serialize)]
pub struct EquityUpdate {
    time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[derive(Clone)]
pub struct EquityChartServer {
    equity_data: Arc<Mutex<Vec<EquityUpdate>>>,
    current_candle: Arc<Mutex<Option<EquityUpdate>>>,
}

impl EquityChartServer {
    pub fn new() -> Self {
        EquityChartServer {
            equity_data: Arc::new(Mutex::new(Vec::new())),
            current_candle: Arc::new(Mutex::new(None)),
        }
    }

    // Update equity and manage candles
    pub fn update_equity(&self, value: f64) {
        let timestamp = Utc::now().timestamp();
        let ten_sec_timestamp = timestamp - (timestamp % 10); // Round to nearest 10 seconds
        
        let mut current_candle = self.current_candle.lock().unwrap();
        
        match &mut *current_candle {
            Some(candle) if candle.time == ten_sec_timestamp => {
                // Update existing candle
                candle.high = candle.high.max(value);
                candle.low = candle.low.min(value);
                candle.close = value;
            }
            _ => {
                // Create new candle
                if let Some(completed_candle) = current_candle.take() {
                    let mut data = self.equity_data.lock().unwrap();
                    data.push(completed_candle);
                }

                *current_candle = Some(EquityUpdate {
                    time: ten_sec_timestamp,
                    open: value,
                    high: value,
                    low: value,
                    close: value,
                });
            }
        }
    }

    pub async fn start_server(&self, port: u16) {
        let equity = self.equity_data.clone();
        let current = self.current_candle.clone();
        
        // Add CORS support
        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["GET", "POST"])
            .allow_headers(vec!["Content-Type"]);
        
        let ws_route = warp::path("ws")
            .and(warp::ws())
            .map(move |ws: warp::ws::Ws| {
                let equity = equity.clone();
                let current = current.clone();
                ws.on_upgrade(move |websocket| handle_connection(websocket, equity, current))
            });

        let routes = ws_route.with(cors);
        
        println!("Chart server running at http://localhost:{}", port);
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }
}

async fn handle_connection(
    ws: warp::ws::WebSocket,
    equity: Arc<Mutex<Vec<EquityUpdate>>>,
    current: Arc<Mutex<Option<EquityUpdate>>>
) {
    let (mut tx, _) = ws.split();
    
    loop {
        // Send both historical and current candle data
        let data = {
            let mut all_data = equity.lock().unwrap().clone();
            if let Some(current_candle) = current.lock().unwrap().as_ref() {
                all_data.push(current_candle.clone());
            }
            serde_json::to_string(&all_data).unwrap()
        };
        
        if tx.send(warp::ws::Message::text(data)).await.is_err() {
            break;
        }
        
        sleep(Duration::from_millis(100)).await;
    }
}
