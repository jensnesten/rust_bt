// core backtesting engine implementation
#[allow(unused_imports)]
use crate::util::as_str;
#[allow(unused_imports)]
use std::cmp::Ordering;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::UnboundedReceiver;
use std::collections::HashMap;

// Define custom error for order margin check.
#[derive(Debug)]
pub enum OrderError {
    MarginExceeded, // error if order notional exceeds available buying power
    FractionalOrderNotAllowed, // error for fractional orders when not using leverage
    TradeLimitExceeded, // error if new order would exceed allowed concurrent positions per side
}

/// A single tick snapshot for one instrument.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickSnapshot {
    pub instrument: String,
    pub date: String,
    pub ask: f64,
    pub bid: f64,
}

/// Hybrid live data: keeps a full history of ticks as well as a current snapshot per instrument.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveData {
    pub ticks: Vec<TickSnapshot>,
    pub current: HashMap<String, TickSnapshot>,
}

/// Order now uses a String to identify the instrument.
#[derive(Clone, Debug)]
pub struct Order {
    // positive size indicates a long order, negative a short
    pub size: f64,
    pub limit: Option<f64>,
    pub stop: Option<f64>,
    pub sl: Option<f64>,
    pub tp: Option<f64>,
    // for contingent orders (sl/tp), parent_trade indicates which trade they relate to (by index)
    pub parent_trade: Option<usize>,
    pub instrument: String,
}

/// Trade now uses a String to identify the instrument.
#[derive(Clone)]
pub struct Trade {
    pub instrument: String,
    pub size: f64,
    pub entry_price: f64,
    pub entry_index: usize,
    pub exit_price: Option<f64>,
    pub exit_index: Option<usize>,
    // optional indices of contingent orders assigned to this trade
    pub sl_order: Option<usize>,
    pub tp_order: Option<usize>,
}

impl Trade {
    // compute profit or loss in cash units for this trade
    pub fn pnl(&self) -> f64 {
        if let Some(exit_price) = self.exit_price {
            self.size * (exit_price - self.entry_price)
        } else {
            0.0
        }
    }
    // compute percent return of this trade
    pub fn pl_pct(&self) -> f64 {
        let exit = self.exit_price.unwrap_or(self.entry_price);
        if self.entry_price != 0.0 {
            (exit / self.entry_price - 1.0) * self.size.signum()
        } else {
            0.0
        }
    }
    // helper method for closing a trade
    pub fn close(&mut self, index: usize, price: f64) {
        self.exit_price = Some(price);
        self.exit_index = Some(index);
    }
}

// current open position can be derived from active trades
pub struct Position;

impl Position {
    // compute net position size from active trades
    pub fn size(trades: &[Trade]) -> f64 {
        trades.iter().map(|t| t.size).sum()
    }
    
    // compute profit/loss of current open position based on current price
    pub fn pl(trades: &[Trade], current_price: f64) -> f64 {
        trades.iter().map(|t| {
            if t.size > 0.0 {
                (current_price - t.entry_price) * t.size
            } else {
                (t.entry_price - current_price) * (-t.size)
            }
        }).sum()
    }
}

/// The live broker uses our hybrid LiveData.
pub struct LiveBroker {
    pub live_data: LiveData,
    pub live_cash: f64,
    pub live_margin: f64,     // margin ratio (0 < margin <= 1)
    pub live_trade_on_close: bool,
    pub live_hedging: bool,
    pub live_exclusive_orders: bool,
    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,      // active trades
    pub closed_trades: Vec<Trade>,
    // equity curve per tick
    pub live_equity: Vec<f64>,
    pub live_max_margin_usage: f64, // track maximum margin usage (percentage)
    pub live_base_equity: f64,      // initial equity for scaling purposes
    pub live_scaling_enabled: bool, // flag to enable scaling
    pub live_margin_usage_history: Vec<f64>, // track historical margin usage
    max_live_concurrent_trades: usize,
}

impl LiveBroker {
    const MARGIN_CALL_THRESHOLD: f64 = 0.85; // 85% margin usage triggers margin call

    pub fn new(
        live_data: LiveData,
        live_cash: f64,
        live_margin: f64,
        live_trade_on_close: bool,
        live_hedging: bool,
        live_exclusive_orders: bool,
        live_scaling_enabled: bool,
    ) -> Self {
        let n = live_data.ticks.len();
        LiveBroker {
            live_data,
            live_cash,
            live_margin,
            live_trade_on_close,
            live_hedging,
            live_exclusive_orders,
            orders: Vec::new(),
            trades: Vec::new(),
            closed_trades: Vec::new(),
            live_equity: vec![live_cash; n],
            live_max_margin_usage: 0.0,
            live_base_equity: live_cash,
            live_scaling_enabled,
            live_margin_usage_history: vec![0.0],
            max_live_concurrent_trades: 0,
        }
    }

    // new_order: place a new order into the live orders queue
    pub fn new_order(&mut self, mut order: Order, current_price: f64) -> Result<(), OrderError> {
        // check fractional orders if no leverage
        if self.live_margin >= 1.0 && order.size.fract() != 0.0 {
            return Err(OrderError::FractionalOrderNotAllowed);
        }
        // scale order size if scaling is enabled
        if self.live_scaling_enabled {
            order.size = self.scale_order_size(order.size);
        }
      
        // check for sufficient buying power
        let order_notional = order.size.abs() * current_price;
        let available = self.available_buying_power();
        if order_notional > available {
            return Err(OrderError::MarginExceeded);
        }
        // enforce trade limits (max three open trades per side) for non-contingent orders
        if order.parent_trade.is_none() {
            if order.size > 0.0 {
                let count = self.trades.iter().filter(|trade| trade.size > 0.0 && trade.exit_price.is_none()).count();
                if count >= 3 {
                    return Err(OrderError::TradeLimitExceeded);
                }
            } else if order.size < 0.0 {
                let count = self.trades.iter().filter(|trade| trade.size < 0.0 && trade.exit_price.is_none()).count();
                if count >= 3 {
                    return Err(OrderError::TradeLimitExceeded);
                }
            }
        }
        // if exclusive orders are enabled, clear any existing orders and trades
        if self.live_exclusive_orders {
            self.orders.clear();
            self.trades.clear();
        }
        if order.parent_trade.is_some() {
            self.orders.insert(0, order);
        } else {
            self.orders.push(order);
        }
        self.update_max_margin_usage();
        self.update_margin_usage();
        Ok(())
    }

    // process_orders: check and execute orders using current live bid and ask prices.
    // For each order, we look up the current snapshot by instrument.
    pub fn process_orders(&mut self, _index: usize) {
        let mut executed_order_indices: Vec<usize> = Vec::new();

        for (i, order) in self.orders.iter_mut().enumerate() {
            // Look up current snapshot for the order's instrument.
            if let Some(current_tick) = self.live_data.current.get(&order.instrument) {
                let current_ask = current_tick.ask;
                let current_bid = current_tick.bid;

                // Handle stop orders.
                if let Some(stop_price) = order.stop {
                    let is_stop_hit = if order.parent_trade.is_some() {
                        // contingent order: for long, trigger if current bid <= stop;
                        // for short, if current ask >= stop.
                        if order.size > 0.0 {
                            current_bid <= stop_price
                        } else {
                            current_ask >= stop_price
                        }
                    } else {
                        // stop entry order: for long, trigger when ask >= stop;
                        // for short, when bid <= stop.
                        if order.size > 0.0 {
                            current_ask >= stop_price
                        } else {
                            current_bid <= stop_price
                        }
                    };
                    if is_stop_hit {
                        order.stop = None; // clear stop to treat as market order.
                    } else {
                        continue;
                    }
                }
                // Handle limit orders.
                if let Some(limit_price) = order.limit {
                    let is_limit_hit = if order.size > 0.0 {
                        current_ask <= limit_price
                    } else {
                        current_bid >= limit_price
                    };
                    if is_limit_hit {
                        executed_order_indices.push(i);
                    } else {
                        continue;
                    }
                } else {
                    // Market order: execute immediately.
                    executed_order_indices.push(i);
                }
            }
        }

        // Clone orders to execute and remove them from the queue in descending order.
        let orders_to_execute: Vec<Order> = executed_order_indices.iter().map(|&i| self.orders[i].clone()).collect();
        executed_order_indices.sort_unstable_by(|a, b| b.cmp(a));
        for i in executed_order_indices {
            self.orders.remove(i);
        }

        for order in orders_to_execute.iter() {
            // Get the current snapshot for this order.
            if let Some(current_tick) = self.live_data.current.get(&order.instrument) {
                let entry_price = if order.size > 0.0 { current_tick.ask } else { current_tick.bid };

                let trade = Trade {
                    size: order.size,
                    entry_price,
                    entry_index: 0, // For live trading you may record a tick counter or timestamp.
                    exit_price: None,
                    exit_index: None,
                    sl_order: None,
                    tp_order: None,
                    instrument: order.instrument.clone(),
                };
                self.trades.push(trade);

                if order.size > 0.0 {
                    println!("open long on {}: {}", order.instrument, entry_price);
                } else {
                    println!("open short on {}: {}", order.instrument, entry_price);
                }

                // If a stop loss is provided, create a contingent order.
                if let Some(sl_value) = order.sl {
                    let trade_idx = self.trades.len() - 1; // index of new trade
                    let contingent_order = Order {
                        size: order.size,
                        limit: None,
                        stop: Some(sl_value),
                        sl: None,
                        tp: order.tp,
                        parent_trade: Some(trade_idx),
                        instrument: order.instrument.clone(),
                    };
                    self.orders.push(contingent_order);
                    if order.size > 0.0 {
                        println!("{} long stop loss set at: {}", order.instrument, sl_value);
                    } else {
                        println!("{} short stop loss set at: {}", order.instrument, sl_value);
                    }
                }
            }
        }
    }

    // update_equity: recalc live equity = live_cash + pnl from open trades.
    // For each trade, we look up the latest price from the current snapshot.
    pub fn update_equity(&mut self, _index: usize) {
        let pnl_sum: f64 = self.trades.iter().map(|trade| {
            if let Some(current_tick) = self.live_data.current.get(&trade.instrument) {
                if trade.size > 0.0 {
                    (current_tick.bid - trade.entry_price) * trade.size
                } else {
                    (trade.entry_price - current_tick.ask) * (-trade.size)
                }
            } else {
                0.0
            }
        }).sum();
        let equity_value = self.live_cash + pnl_sum;
        self.live_equity.push(equity_value);
    }

    // close_position: close one open trade using the current live prices.
    pub fn close_position(&mut self, trade_index: usize, _index: usize) {
        if trade_index >= self.trades.len() {
            return;
        }
        let trade = self.trades.remove(trade_index);
        if let Some(current_tick) = self.live_data.current.get(&trade.instrument) {
            let exit_price = if trade.size > 0.0 { current_tick.bid } else { current_tick.ask };
            let closed_trade = Trade {
                size: trade.size,
                entry_price: trade.entry_price,
                entry_index: trade.entry_index,
                exit_price: Some(exit_price),
                exit_index: Some(0),
                sl_order: trade.sl_order,
                tp_order: trade.tp_order,
                instrument: trade.instrument.clone(),
            };
            self.live_cash += closed_trade.pnl();
            self.closed_trades.push(closed_trade);
            if trade.size > 0.0 {
                println!("closed long on {}: {}", trade.instrument, exit_price);
            } else {
                println!("closed short on {}: {}", trade.instrument, exit_price);
            }
        }
    }

    // close_all_trades: liquidate all open trades at current live prices.
    pub fn close_all_trades(&mut self, _index: usize) {
        let mut total_pnl = 0.0;
        let trades: Vec<_> = self.trades.drain(..).collect();
        for trade in trades {
            if let Some(current_tick) = self.live_data.current.get(&trade.instrument) {
                let exit_price = if trade.size > 0.0 { current_tick.bid } else { current_tick.ask };
                let closed_trade = Trade {
                    size: trade.size,
                    entry_price: trade.entry_price,
                    entry_index: trade.entry_index,
                    exit_price: Some(exit_price),
                    exit_index: Some(0),
                    sl_order: trade.sl_order,
                    tp_order: trade.tp_order,
                    instrument: trade.instrument.clone(),
                };
                total_pnl += closed_trade.pnl();
                self.closed_trades.push(closed_trade);
                if trade.size > 0.0 {
                    println!("closed long on {}: {}", trade.instrument, exit_price);
                } else {
                    println!("closed short on {}: {}", trade.instrument, exit_price);
                }
            }
        }
        self.live_cash += total_pnl;
        self.orders.clear();
    }

    // next: process one tick of live data.
    // In a backtest this could be called for each new tick, but here we assume that current prices come from the `current` snapshot.
    pub fn next(&mut self, index: usize) {
        self.max_live_concurrent_trades = self.max_live_concurrent_trades.max(self.trades.len());
        self.process_orders(index);
        self.update_equity(index);
        self.check_margin_call(index);
        if *self.live_equity.last().unwrap_or(&self.live_cash) <= 0.0 {
            self.close_all_trades(index);
            self.live_cash = 0.0;
            // Reset the equity history.
            self.live_equity.push(0.0);
        }
        self.update_margin_usage();
    }

    // check_margin_call: force liquidation if margin usage exceeds threshold.
    fn check_margin_call(&mut self, index: usize) {
        let usage = self.current_margin_usage();
        if usage > Self::MARGIN_CALL_THRESHOLD {
            println!("// margin call triggered at {:.2}% usage", usage * 100.0);
            self.close_all_trades(index);
            self.update_margin_usage();
        }
    }

    pub fn available_buying_power(&self) -> f64 {
        (self.live_cash / self.live_margin) - self.current_exposure()
    }

    pub fn current_exposure(&self) -> f64 {
        self.trades.iter().map(|trade| trade.size.abs() * trade.entry_price).sum()
    }

    pub fn current_margin_usage(&self) -> f64 {
        if (self.live_margin - 1.0).abs() < std::f64::EPSILON {
            return 0.0;
        }
        let total_allowed = self.live_cash / self.live_margin;
        if total_allowed > 0.0 {
            self.current_exposure() / total_allowed
        } else {
            0.0
        }
    }

    pub fn update_max_margin_usage(&mut self) {
        let usage = self.current_margin_usage();
        if usage > self.live_max_margin_usage {
            self.live_max_margin_usage = usage;
        }
    }

    pub fn scale_order_size(&self, base_size: f64) -> f64 {
        let current_equity = *self.live_equity.last().unwrap_or(&self.live_cash);
        base_size * (current_equity / self.live_base_equity)
    }

    pub fn update_margin_usage(&mut self) {
        let usage = self.current_margin_usage();
        if usage > self.live_max_margin_usage {
            self.live_max_margin_usage = usage;
        }
        self.live_margin_usage_history.push(usage);
    }

    // new method to print basic live trading stats in one console line.
    pub fn print_live_stats(&self, tick: usize) {
        println!(
            "\n tick: {} | cash: {:.2} | open trades: {} | closed trades: {} | equity: {:.2} | margin usage: {:.2}% \n",
            tick,
            self.live_cash,
            self.trades.len(),
            self.closed_trades.len(),
            self.live_equity.last().unwrap_or(&self.live_cash),
            self.current_margin_usage() * 100.0
        );
    }
}

/// Strategy trait remains similar.
pub trait LiveStrategy {
    fn init(&mut self, broker: &mut LiveBroker, data: &LiveData);
    fn next(&mut self, broker: &mut LiveBroker, index: usize);
}

pub type LiveStrategyRef = Box<dyn LiveStrategy>;

/// The backtest driver.
pub struct LiveBacktest {
    pub data: LiveData,
    pub broker: LiveBroker,
    pub strategy: LiveStrategyRef,
}

impl LiveBacktest {
    pub fn new(
        live_data: LiveData,
        live_strategy: LiveStrategyRef,
        live_cash: f64,
        live_margin: f64,
        live_trade_on_close: bool,
        live_hedging: bool,
        live_exclusive_orders: bool,
        live_scaling_enabled: bool,
    ) -> Self {
        let broker = LiveBroker::new(
            live_data.clone(),
            live_cash,
            live_margin,
            live_trade_on_close,
            live_hedging,
            live_exclusive_orders,
            live_scaling_enabled,
        );
        LiveBacktest {
            data: live_data,
            broker,
            strategy: live_strategy,
        }
    }

    // The run method now expects incoming LiveData (hybrid type).
    // For each incoming snapshot, we append its ticks to our history and update the current snapshot.
    pub async fn run(&mut self, mut rx: UnboundedReceiver<LiveData>) {
        // init strategy with initial live data
        self.strategy.init(&mut self.broker, &self.data);
        let mut tick: usize = self.broker.live_data.ticks.len();
        while let Some(new_data) = rx.recv().await {
            // Append incoming ticks to the history.
            self.broker.live_data.ticks.extend(new_data.ticks.iter().cloned());
            // Update the current snapshot for each tick.
            for tick_snapshot in new_data.ticks.iter() {
                self.broker
                    .live_data
                    .current
                    .insert(tick_snapshot.instrument.clone(), tick_snapshot.clone());
            }
            // Determine the new tick count.
            let new_tick_count = self.broker.live_data.ticks.len();
            // Process each newly appended tick.
            for _ in tick..new_tick_count {
                self.strategy.next(&mut self.broker, tick);
                self.broker.next(tick);
                self.broker.print_live_stats(tick);
                tick += 1;
            }
        }
    }
}
