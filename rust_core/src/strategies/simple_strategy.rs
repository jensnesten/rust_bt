use crate::engine::{Broker, OhlcData, Order, Strategy};
pub struct SimpleStrategy;


impl SimpleStrategy {
    pub fn new() -> Self {
        SimpleStrategy
    }
}

impl Strategy for SimpleStrategy {
    fn init(&mut self, _broker: &mut Broker, _data: &OhlcData) {
        // initialization can precompute indicators, etc..

    }

    fn next(&mut self, broker: &mut Broker, index: usize) {
        let size = broker.cash / broker.data.close[index];
        // buy at first closing price, and sell at the last
        if broker.trades.is_empty() {
            let order = Order {
                size: size,
                limit: None,
                stop: None,
                sl: None,
                tp: None,
                parent_trade: None,
                instrument: 1,
            };
            if let Err(_e) = broker.new_order(order, broker.data.close[index]) {
                // handle error - for example, you could print a warning or skip the order
                // (error: margin_exceeded)
            }
            println!("Buy at {}", broker.data.close[index]); 
        } else if index == broker.data.close.len() - 1 {   
            // we're at the last candle, close all positions
            broker.close_position(0, index);
            println!("Sell at {}", broker.data.close[index]);
        }
    }
}
