use crate::live_engine::{LiveBroker, LiveData, Order, LiveStrategy};
use crate::position::PositionManager;

pub struct LiveStatArbSpreadStrategy {
    pub size: f64,
    pub lookback: usize,
    pub zscore_threshold: f64,
    pub stop_loss: f64,
    pub spread: Vec<f64>,
    pub bid: Vec<f64>,
    pub ask: Vec<f64>,
    pub positions: PositionManager,
}

impl LiveStatArbSpreadStrategy {
    pub fn new() -> Self {
        LiveStatArbSpreadStrategy {
            size: 50.0,
            lookback: 20,
            zscore_threshold: 1.2,
            stop_loss: 50.0 * 0.0075,
            spread: Vec::new(),
            bid: Vec::new(),
            ask: Vec::new(),
            positions: PositionManager::new(4),  // allow max 3 positions per side
        }
    }
}

impl LiveStrategy for LiveStatArbSpreadStrategy {
    fn init(&mut self, _broker: &mut LiveBroker, _data: &LiveData) {
        // nothing to do; strategy will use broker's live data directly
    }


    fn next(&mut self, broker: &mut LiveBroker, index: usize) {
        // get live data and copy price values to avoid borrow conflicts
        
        let instrument = &broker.live_data.current.get("US500").unwrap().instrument;
        
     
        
        // copy live prices (f64 is copy) to prevent borrow conflict
        let current_ask = &broker.live_data.current.get("US500").unwrap().ask;
        let current_bid = &broker.live_data.current.get("US500").unwrap().bid;

        println!("instrument - Uic: {}", instrument);
        println!("current_ask: {}, current_bid: {}", current_ask, current_bid);
        
        // calculate current spread using local prices
        //let current_log_spread = current_ask.ln() - current_bid.ln();
        let current_log_spread = ((current_ask.ln() + current_bid.ln()) / 2.0).ln();
        
        // push current spread and maintain window size
        self.spread.push(current_log_spread);
        if self.spread.len() > 10 {
            self.spread.remove(0);
        }

        // ensure enough data to compute standard deviation to avoid underflow
        if self.spread.len() < 2 {
            return;
        }

        let spread_mean = self.spread.iter().sum::<f64>() / self.spread.len() as f64;
        let spread_std = (self.spread.iter()
            .map(|x| (x - spread_mean).powi(2))
            .sum::<f64>() / ((self.spread.len() - 1) as f64))
            .sqrt();
        let zscore = (current_log_spread - spread_mean) / spread_std;


        // short when zscore is high (overvalued)
        if zscore > self.zscore_threshold && broker.current_margin_usage() < 0.65 {
            let order = Order {
                size: -self.size,
                sl: Some(current_ask + self.stop_loss),
                tp: None,
                limit: None,
                stop: None,
                parent_trade: None,
                instrument: "US500".to_string(),
            };
            if let Err(_e) = broker.new_order(order, current_ask.clone()) {
                // error handling (e.g., print warning)
            }
            self.positions.register_position(-self.size);
            //println!("short at {} (zscore: {})", current_ask, zscore);
        }
        // long when zscore is low (undervalued)
        else if zscore < -self.zscore_threshold && broker.current_margin_usage() < 0.65{
            let order = Order {
                size: self.size,
                sl: Some(current_bid - self.stop_loss),
                tp: None,
                limit: None,
                stop: None,
                parent_trade: None,
                instrument: "US500".to_string(),
            };  
            if let Err(_e) = broker.new_order(order, current_bid.clone()) {
                // error handling (e.g., print warning)
            }
            self.positions.register_position(self.size);

        } else if zscore.abs() < self.zscore_threshold / 2.0 && !self.positions.is_empty() {
            // close trades only if positions exist; use mid price as exit price
            broker.close_all_trades(index); // update broker to accept close_price

        }

        // handle stop losses by checking recently closed trades
        for trade in broker.closed_trades.iter().skip(broker.closed_trades.len().saturating_sub(1)) {
            if trade.exit_index == Some(index) {
                self.positions.close_position(trade.size);
                
            }
        }
    }
}