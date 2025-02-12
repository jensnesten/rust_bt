use crate::engine::{LiveBroker, LiveData, Order, LiveStrategy};
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
            size: 20.0,
            lookback: 10,
            zscore_threshold: 1.2,
            stop_loss: 5.0 * 0.0075,
            spread: Vec::new(),
            bid: Vec::new(),
            ask: Vec::new(),
            positions: PositionManager::new(2),  // allow max 3 positions per side
        }
    }

    fn calculate_log_spread(&self, index: usize) -> f64 {
        self.ask[index].ln() - self.bid[index].ln()
    }
}

impl LiveStrategy for LiveStatArbSpreadStrategy {
    fn init(&mut self, _broker: &mut LiveBroker, live_data: &LiveData) {
        self.bid = live_data.bid.clone();
        self.ask = live_data.ask.clone();
    }

    fn next(&mut self, broker: &mut LiveBroker, index: usize) {
        if index < self.lookback || index >= self.ask.len() {
            return;
        }

        let current_spread = self.calculate_log_spread(index);
        self.spread.push(current_spread);
        if self.spread.len() > self.lookback {
            self.spread.remove(0);
        }

        let spread_mean = self.spread.iter().sum::<f64>() / self.spread.len() as f64;
        let spread_std = (self.spread.iter()
            .map(|x| (x - spread_mean).powi(2))
            .sum::<f64>() / ((self.spread.len() - 1) as f64))
            .sqrt();
        let zscore = (current_spread - spread_mean) / spread_std;


        // short when zscore is high (overvalued)
        if self.positions.can_open_short() && zscore > self.zscore_threshold {
            let order = Order {
                size: -self.size,
                sl: Some(self.ask[index] + (self.stop_loss + broker.live_bidask_spread)),
                tp: None,
                limit: None,
                stop: None,
                parent_trade: None,
                instrument: 1,
            };
            if let Err(_e) = broker.new_order(order, self.ask[index]) {
                // handle error - for example, you could print a warning or skip the order
                // (error: margin_exceeded)
            }
            self.positions.register_position(-self.size);
            //println!("short at {} (zscore: {})", price, zscore);
        }
        // long when zscore is low (undervalued)
        else if self.positions.can_open_long() && zscore < -self.zscore_threshold {
            let order = Order {
                size: self.size,
                sl: Some(self.bid[index] - (self.stop_loss + broker.live_bidask_spread)),
                tp: None,
                limit: None,
                stop: None,
                parent_trade: None,
                instrument: 1,
            };  
            if let Err(_e) = broker.new_order(order, self.ask[index]) {
                // handle error - for example, you could print a warning or skip the order
                // (error: margin_exceeded)
            }
            self.positions.register_position(self.size);
            //println!("long at {} (zscore: {})", price, zscore);

        } else if zscore.abs() < self.zscore_threshold / 2.0 {
            // close all trades
            broker.close_all_trades(index);
        }

        // handle stop losses by checking recently closed trades
        for trade in broker.live_closed_trades.iter().skip(broker.live_closed_trades.len().saturating_sub(1)) {
            if trade.exit_index == Some(index) {
                self.positions.close_position(trade.size);
            }
        }
    }
}