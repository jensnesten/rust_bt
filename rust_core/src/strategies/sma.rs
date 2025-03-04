use crate::engine::{Broker, OhlcData, Order, Strategy, Trade};


pub struct SmaStrategy {
    sma_period: usize,
    sma_period_2: usize,
    close: Vec<f64>,
}

impl SmaStrategy {
    pub fn new() -> Self {
        SmaStrategy {
            sma_period: 10,
            sma_period_2: 20,
            close: Vec::new(),
        }
    }
}

impl Strategy for SmaStrategy {
    fn init(&mut self, _broker: &mut Broker, data: &OhlcData) {
        self.close = data.close.clone();
    }

    fn next(&mut self, broker: &mut Broker, index: usize) {
        // ensure we have enough data to compute both current and previous moving averages
        let min_required = self.sma_period.max(self.sma_period_2) + 1;
        if index < min_required { return; }

        
        let window1_current: f64 = self.close[index - self.sma_period..index]
            .iter().sum::<f64>() / self.sma_period as f64;
        let window2_current: f64 = self.close[index - self.sma_period_2..index]
            .iter().sum::<f64>() / self.sma_period_2 as f64;
        let curr_diff = window1_current - window2_current;
        
        let window1_prev: f64 = self.close[index - 1 - self.sma_period..index - 1]
            .iter().sum::<f64>() / self.sma_period as f64;
        let window2_prev: f64 = self.close[index - 1 - self.sma_period_2..index - 1]
            .iter().sum::<f64>() / self.sma_period_2 as f64;
        let prev_diff = window1_prev - window2_prev;
        let price = self.close[index];

        
        if prev_diff <= 0.0 && curr_diff > 0.0 {
            // bullish cross: only buy when the difference switches from non-positive to positive
            let order = Order {
                size: 30.0,
                tp: None,
                sl: None,
                limit: None,
                stop: None,
                parent_trade: None,
                instrument: 1,
            };
            if let Err(_e) = broker.new_order(order, price) {
                // handle error - for example, you could print a warning or skip the order
                // (error: margin_exceeded)
            }
            println!("Buy at {}", self.close[index]);

        } else if prev_diff >= 0.0 && curr_diff < 0.0 && broker.trades.len() > 0 {
            let trade = broker.trades.remove(0);
            let closed_trade = Trade {
                size: trade.size,
                entry_price: trade.entry_price,
                entry_index: trade.entry_index,
                exit_price: Some(self.close[index]),
                exit_index: Some(index),
                sl_order: trade.sl_order,
                tp_order: trade.tp_order,
                instrument: trade.instrument,
            };
            broker.closed_trades.push(closed_trade);
            println!("Closed at {}", self.close[index]);
 
        } 

    }
    
}
        