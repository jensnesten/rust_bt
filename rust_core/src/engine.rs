// core backtesting engine implementation
#[allow(unused_imports)]
use crate::util::as_str;
#[allow(unused_imports)]
use std::cmp::Ordering;

// import chrono and the plot module
use chrono::NaiveDateTime;
use crate::plot::plot_equity;
use crate::plot::plot_equity_and_benchmark;
use crate::plot::plot_margin_usage;

// define custom error for order margin check
#[derive(Debug)]
pub enum OrderError {
    MarginExceeded, // error if order notional exceeds available buying power
    FractionalOrderNotAllowed, // new error type for fractional orders when not using leverage
    TradeLimitExceeded, // error if new order would exceed allowed concurrent positions per side
}

#[derive(Clone, Debug)]
pub struct OhlcData {
    // ohlc data vectors; index is assumed to be ticks (for example, daily bars)
    pub date: Vec<String>,
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub close2: Vec<f64>,
    pub volume: Option<Vec<f64>>,
}

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
    // instrument flag: 1 = primary (using Close), 2 = hedge (using Close2)
    pub instrument: u8,
}

#[derive(Clone)]
pub struct Trade {
    pub instrument: u8,
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
    // add helper method to Trade struct for cleaner code
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

// broker manages orders, trades, cash and the equity curve
pub struct Broker {
    pub data: OhlcData,
    pub cash: f64,
    pub bidask_spread: f64,
    pub commission: f64, // commission ratio (e.g. 0.001 means 0.1% fee)
    pub margin: f64,     // margin ratio (0 < margin <= 1)
    pub trade_on_close: bool,
    pub hedging: bool,
    pub exclusive_orders: bool,
    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,      // active trades
    pub closed_trades: Vec<Trade>,
    // equity curve per tick
    pub equity: Vec<f64>,
    pub max_margin_usage: f64, // track maximum margin usage (percentage)
    pub base_equity: f64,      // initial equity for scaling purposes
    pub scaling_enabled: bool, // flag to enable scaling
    pub margin_usage_history: Vec<f64>, // track historical margin usage
    max_concurrent_trades: usize,
}

impl Broker {
    const MARGIN_CALL_THRESHOLD: f64 = 0.90; // 90% margin usage triggers margin call

    pub fn new(
        data: OhlcData,
        cash: f64,
        commission: f64,
        bidask_spread: f64,
        margin: f64,
        trade_on_close: bool,
        hedging: bool,
        exclusive_orders: bool,
        scaling_enabled: bool,
    ) -> Self {
        let n = data.close.len();
        Broker {
            data,
            cash,
            bidask_spread,
            commission,
            margin,
            trade_on_close,
            hedging,
            exclusive_orders,
            orders: Vec::new(),
            trades: Vec::new(),
            closed_trades: Vec::new(),
            equity: vec![cash; n],
            max_margin_usage: 0.0,
            base_equity: cash,
            scaling_enabled,
            margin_usage_history: vec![0.0],
            max_concurrent_trades: 0,
        }
    }

    pub fn current_exposure(&self) -> f64 {
        self.trades.iter().map(|trade| trade.size.abs() * trade.entry_price).sum()
    }
    
    // compute price adjusted for commission and bidask spread.
    // for long orders (size > 0), the adjusted price is: price * (1 + commission) + bidask_spread.
    // for short orders (size < 0), the adjusted price is: price * (1 - commission) - bidask_spread.
    // if size is zero, the price is unchanged.
    pub fn adjusted_price(&self, size: f64, price: f64) -> f64 {
        // apply commission adjustment
        let price_with_commission = price * (1.0 + size.signum() * self.commission);
        // always apply bidask spread if set; note bidask spread is a fixed 0.5 usd per trade
        if self.bidask_spread > 0.0 {
            if size > 0.0 {
                price_with_commission + self.bidask_spread
            } else if size < 0.0 {
                price_with_commission - self.bidask_spread
            } else {
                price_with_commission
            }
        } else {
            price_with_commission
        }
    }
    
    // place a new order
    pub fn new_order(&mut self, mut order: Order, current_price: f64) -> Result<(), OrderError> {
        // prevent fractional orders when not using leverage
        if self.margin >= 1.0 && order.size.fract() != 0.0 {
            return Err(OrderError::FractionalOrderNotAllowed);
        }

        // if scaling is enabled, adjust order size
        if self.scaling_enabled {
            order.size = self.scale_order_size(order.size);
        }
        
        // adjust order size for hedge instrument (instrument 2) dynamically based on price ratio:
        // factor = (current primary price) / (current hedge price)
        if order.instrument == 2 {
            let last_tick = self.equity.len().saturating_sub(1);
            let primary_price = self.data.close[last_tick];
            let hedge_price = self.data.close2[last_tick];
            let factor = primary_price / hedge_price;
            order.size *= factor;
        }
        
        // calculate order notional using current price
        let order_notional = order.size.abs() * current_price;
        let available = self.available_buying_power();

        // if order exceeds available buying power, return error
        if order_notional > available {
            return Err(OrderError::MarginExceeded);
        }
        
        // enforce trade limit on new (non-contingent) orders; allow max 3 per side
        if order.parent_trade.is_none() {
            if order.size > 0.0 {
                // count active long trades
                let count = self.trades.iter().filter(|trade| trade.size > 0.0 && trade.exit_price.is_none()).count();
                if count >= 3 {
                    return Err(OrderError::TradeLimitExceeded);
                }
            } else if order.size < 0.0 {
                // count active short trades
                let count = self.trades.iter().filter(|trade| trade.size < 0.0 && trade.exit_price.is_none()).count();
                if count >= 3 {
                    return Err(OrderError::TradeLimitExceeded);
                }
            }
        }
        // clear orders if exclusive orders are enabled
        if self.exclusive_orders {
            self.orders.clear();
            self.trades.clear();
        }
        if order.parent_trade.is_some() {
            self.orders.insert(0, order);
        } else {
            self.orders.push(order);
        }

        // update max margin usage stat
        self.update_max_margin_usage();

        // update margin usage history
        self.update_margin_usage();

        Ok(())
    }
    

    // updated close_position method with separate trade_index and tick_index parameters
    pub fn close_position(&mut self, trade_index: usize, tick_index: usize) {
        // check if the specified trade index is valid
        if trade_index < self.trades.len() {
            let trade = self.trades.remove(trade_index);
            // create a closed trade using the market price from the specified tick_index
            let raw_exit_price = if trade.instrument == 1 {
                self.data.close[tick_index]
            } else {
                self.data.close2[tick_index]
            };
            let closed_trade = Trade {
                size: trade.size,
                entry_price: trade.entry_price,
                entry_index: trade.entry_index,
                exit_price: Some(self.adjusted_price(trade.size, raw_exit_price)),
                exit_index: Some(tick_index),
                sl_order: trade.sl_order,
                tp_order: trade.tp_order,
                instrument: trade.instrument,
            };
            // update the broker's cash balance with the profit or loss from the closed trade
            self.cash += closed_trade.pnl();
            // push the closed trade into the closed_trades vector
            self.closed_trades.push(closed_trade);
        }
    }

    // Revised method for closing all trades, using separate tick indices per instrument.
    // tick1 is used for instrument 1 and tick2 for instrument 2.
    pub fn close_all_trades(&mut self, tick1: usize, tick2: usize) {
        // Extract local references to avoid borrow conflicts.
        let close_prices = &self.data.close;
        let close2_prices = &self.data.close2;
        let commission = self.commission;
        let bidask_spread = self.bidask_spread;
        let adjusted_price = |size: f64, price: f64| -> f64 {
            let price_with_commission = price * (1.0 + size.signum() * commission);
            if bidask_spread > 0.0 {
                if size > 0.0 {
                    price_with_commission + bidask_spread
                } else if size < 0.0 {
                    price_with_commission - bidask_spread
                } else {
                    price_with_commission
                }
            } else {
                price_with_commission
            }
        };

        let mut total_pnl = 0.0;

        // Partition trades by instrument.
        let (mut trades_inst1, mut trades_inst2): (Vec<Trade>, Vec<Trade>) =
            self.trades.drain(..).partition(|trade| trade.instrument == 1);

        // Process instrument 1 trades.
        for mut trade in trades_inst1.drain(..) {
            let raw_exit_price = close_prices[tick1];
            let exit_price = adjusted_price(trade.size, raw_exit_price);
            trade.exit_price = Some(exit_price);
            trade.exit_index = Some(tick1);
            total_pnl += if trade.size > 0.0 {
                (exit_price - trade.entry_price) * trade.size
            } else {
                (trade.entry_price - exit_price) * (-trade.size)
            };
            self.closed_trades.push(trade);
        }

        // Process instrument 2 trades.
        for mut trade in trades_inst2.drain(..) {
            let close2 = close2_prices[tick2];
            let exit_price = adjusted_price(trade.size, close2);
            trade.exit_price = Some(exit_price);
            trade.exit_index = Some(tick2);
            total_pnl += if trade.size > 0.0 {
                (exit_price - trade.entry_price) * trade.size
            } else {
                (trade.entry_price - exit_price) * (-trade.size)
            };
            self.closed_trades.push(trade);
        }

        // Update cash balance.
        self.cash += total_pnl;

        // Cancel any pending orders.
        self.orders.clear();
    }
    
    // process orders at a given tick index based on current market prices
    pub fn process_orders(&mut self, index: usize) {
        let open_price = self.data.open[index];
        let high = self.data.high[index];
        let low = self.data.low[index];
        let prev_close = if index > 0 { self.data.close[index - 1] } else { open_price };

        // for the hedge instrument we assume price is taken from 'Close2'
        let hedge_price = self.data.close2[index];
        let prev_hedge = if index > 0 { self.data.close2[index - 1] } else { hedge_price };

        let mut executed_order_indices: Vec<usize> = Vec::new();
        let reprocess_orders = false;
        
        // check each order in the queue
        for (i, order) in self.orders.iter_mut().enumerate() {
            // check stop order condition
            if let Some(stop_price) = order.stop {
                let is_stop_hit = if order.parent_trade.is_some() {
                    // contingent stop loss order for an open trade:
                    // for a long trade, trigger if current low is below (or equal) to the stop loss price;
                    // for a short trade, trigger if current high is above (or equal) to the stop loss price
                    if order.size > 0.0 {
                        low <= stop_price
                    } else {
                        high >= stop_price
                    }
                } else {
                    // non-contingent stop entry order:
                    // for a long stop entry, trigger when high reaches or exceeds the stop price;
                    // for a short, when low reaches or falls below the stop price.
                    if order.size > 0.0 {
                        high >= stop_price
                    } else {
                        low <= stop_price
                    }
                };
                if is_stop_hit {
                    // on stop, remove the stop price to treat as market order
                    order.stop = None;
                } else {
                    continue;
                }
            }
            // if limit is set, verify limit condition
            if let Some(limit_price) = order.limit {
                let is_limit_hit = if order.size > 0.0 {
                    low < limit_price
                } else {
                    high > limit_price
                };
                if is_limit_hit {
                    executed_order_indices.push(i);
                } else {
                    continue;
                }
            } else {
                // market order: execute immediately using prev_close if trade_on_close, else open price
                executed_order_indices.push(i);
            }
        }
        
        // clone orders to execute then remove them from order queue (process in descending order to avoid index issues)
        let orders_to_execute: Vec<Order> = executed_order_indices.iter().map(|&i| self.orders[i].clone()).collect();
        executed_order_indices.sort_unstable_by(|a, b| b.cmp(a));
        for i in executed_order_indices {
            self.orders.remove(i);
        }
        
        // execute each selected order
        for order in orders_to_execute.iter() {
            let exec_price = if let Some(limit_price) = order.limit {
                limit_price
            } else {
                if order.instrument == 1 {
                    if self.trade_on_close { prev_close } else { open_price }
                } else {
                    if self.trade_on_close { prev_hedge } else { hedge_price }
                }
            };
            let adjusted_price = self.adjusted_price(order.size, exec_price);
            
            if let Some(parent_idx) = order.parent_trade {
                // this is a contingent order (sl/tp)
                if parent_idx < self.trades.len() {
                    let trade = self.trades.remove(parent_idx);
                    let closed_trade = Trade {
                        size: trade.size,
                        entry_price: trade.entry_price,
                        entry_index: trade.entry_index,
                        exit_price: Some(adjusted_price),
                        exit_index: Some(index),
                        sl_order: trade.sl_order,
                        tp_order: trade.tp_order,
                        instrument: trade.instrument,
                    };
                    // Update cash balance when closing trade 
                    // doesnt work for some reason
                    //oh wait i know
                    //no wait it should work
                    self.cash += closed_trade.pnl();
                    self.closed_trades.push(closed_trade);
                }
            } else {
                // stand-alone order: open a new trade
                let trade = Trade {
                    size: order.size,
                    entry_price: adjusted_price,
                    entry_index: index,
                    exit_price: None,
                    exit_index: None,
                    sl_order: None,
                    tp_order: None,
                    instrument: order.instrument,
                };
                self.trades.push(trade);

                // if a stop loss price is provided (in the 'sl' field),
                // create a contingent stop loss order to ensure losses are capped
                if let Some(sl_value) = order.sl {
                    let trade_idx = self.trades.len() - 1; // index of the newly opened trade
                    let contingent_order = Order {
                        size: order.size, // same sign as the original trade
                        limit: None,
                        // store the stop loss price in the 'stop' field for proper triggering
                        stop: Some(sl_value),
                        sl: None,
                        tp: order.tp, // pass through take profit if specified
                        parent_trade: Some(trade_idx),
                        instrument: order.instrument,
                    };
                    self.orders.push(contingent_order);
                }
            }
        }
        
        // if necessary, reprocess orders (for sl/tp orders that might execute in the same tick)
        if reprocess_orders {
            self.process_orders(index);
        }
    }
    
    // update equity at a given tick index; equity = cash + sum(pnl of open trades)
    pub fn update_equity(&mut self, index: usize) {
        let current_close = self.data.close[index];
        let pnl_sum: f64 = self.trades.iter().map(|trade| {
            if trade.size > 0.0 {
                (current_close - trade.entry_price) * trade.size
            } else {
                (trade.entry_price - current_close) * (-trade.size)
            }
        }).sum();
        let equity_value = self.cash + pnl_sum;
        if index < self.equity.len() {
            self.equity[index] = equity_value;
        } else {
            self.equity.push(equity_value);
        }
    }
    
    // add new method to check for and handle margin calls
    fn check_margin_call(&mut self, index: usize) {
        // get current margin usage
        let usage = self.current_margin_usage();
        
        // if margin usage exceeds threshold, force liquidation
        if usage > Self::MARGIN_CALL_THRESHOLD {
            println!("// margin call triggered at {:.2}% usage", usage * 100.0);
            self.close_all_trades(index, index);
            // update margin usage after liquidation
            self.update_margin_usage();
        }
    }

    // modify the next() method to include margin call check
    pub fn next(&mut self, index: usize) {
        // update max_concurrent_trades if current number is higher
        self.max_concurrent_trades = self.max_concurrent_trades.max(self.trades.len());
        
        self.process_orders(index);
        self.update_equity(index);
        
        // check for margin call before equity check
        self.check_margin_call(index);
        
        // if equity drops to zero or below, close all trades and set cash to zero
        if self.equity[index] <= 0.0 {
            self.close_all_trades(index, index);
            self.cash = 0.0;
            for t in index..self.equity.len() {
                self.equity[t] = 0.0;
            }
        }
        
        // update margin usage for every tick
        self.update_margin_usage();
    }

    // calculate available buying power given margin requirements
    pub fn available_buying_power(&self) -> f64 {
        // total allowed notional = cash / margin, subtract current exposure
        (self.cash / self.margin) - self.current_exposure()
    }

    // compute the current margin usage as a fraction of the total allowed notional,
    // but if margin is 1.0 (i.e. no leverage), return 0.0
    pub fn current_margin_usage(&self) -> f64 {
        // no leverage: return 0.0
        if (self.margin - 1.0).abs() < std::f64::EPSILON {
            return 0.0;
        }
        let total_allowed = self.cash / self.margin;
        if total_allowed > 0.0 {
            self.current_exposure() / total_allowed
        } else {
            0.0
        }
    }

    // update the maximum margin usage stat if the current usage is higher
    pub fn update_max_margin_usage(&mut self) {
        let usage = self.current_margin_usage();
        if usage > self.max_margin_usage {
            self.max_margin_usage = usage;
        }
    }

    // compute a scaled order size if scaling is enabled with leverage factor
    pub fn scale_order_size(&self, base_size: f64) -> f64 {
        // scale ordersize by current equity scaling and leverage (1 / margin)
        let current_equity = *self.equity.last().unwrap_or(&self.cash);
        base_size * (current_equity / self.base_equity)
    }

    // update margin usage history whenever position changes and update max margin usage too
    pub fn update_margin_usage(&mut self) {
        let usage = self.current_margin_usage();
        // update max usage if current usage is higher
        if usage > self.max_margin_usage {
            self.max_margin_usage = usage;
        }
        self.margin_usage_history.push(usage);
    }

    // add a method to print trading statistics
    pub fn print_trading_stats(&self) {
        // print max concurrent trades and current open trades
        println!("// max concurrent trades during backtest: {}", self.max_concurrent_trades);
        println!("// current open trades: {}", self.trades.len());
    }

    // new method to print a detailed log of all closed trades
    pub fn print_trade_log(&self) {
        println!("// trade log:");
        for (index, trade) in self.closed_trades.iter().enumerate() {
            println!("trade {}: size: {}, entry: {} at tick {}, exit: {} at tick {}, pnl: {}",
                index,
                trade.size,
                trade.entry_price,
                trade.entry_index.saturating_add(1),
                trade.exit_price.unwrap_or(0.0),
                trade.exit_index.unwrap_or(0).saturating_add(1),
                trade.pnl()
            );
        }
    }

    // new method to save trade log to file
    pub fn save_trade_log(&self, file_path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;
        // open (or create) the file for writing
        let mut file = File::create(file_path)?;
        writeln!(file, "// trade log:")?;
        for (index, trade) in self.closed_trades.iter().enumerate() {
            writeln!(file, "trade {}: size: {}, entry: {} at tick {}, exit: {} at tick {}, pnl: {}",
                index,
                trade.size,
                trade.entry_price,
                trade.entry_index.saturating_add(1),
                trade.exit_price.unwrap_or(0.0),
                trade.exit_index.unwrap_or(0).saturating_add(1),
                trade.pnl()
            )?;
        }
        Ok(())
    }
}
// trait for trading strategies; implementations must provide init and next methods.
pub trait Strategy {
    // initialization where indicators can be precomputed and orders can be declared
    fn init(&mut self, broker: &mut Broker, data: &OhlcData);
    // next is called on every tick, where trading decisions are made
    fn next(&mut self, broker: &mut Broker, index: usize);
}
// alias for user strategies to be boxed for dynamic dispatch
pub type StrategyRef = Box<dyn Strategy>;

// backtest struct ties together data, a broker instance and a strategy instance.
pub struct Backtest {
    pub data: OhlcData,
    pub cash: f64,
    pub broker: Broker,
    pub strategy: StrategyRef,
    pub commission: f64,
    pub bidask_spread: f64,
    pub margin: f64,
    pub trade_on_close: bool,
    pub hedging: bool,
    pub exclusive_orders: bool,
}

impl Backtest {
    pub fn new(
        data: OhlcData,
        strategy: StrategyRef,
        cash: f64,
        commission: f64,
        bidask_spread: f64,
        margin: f64,
        trade_on_close: bool,
        hedging: bool,
        exclusive_orders: bool,
        scaling_enabled: bool,
    ) -> Self {
        let broker = Broker::new(
            data.clone(),
            cash,
            commission,
            bidask_spread,                                                                                                  
            margin,
            trade_on_close,
            hedging,
            exclusive_orders,
            scaling_enabled,
        );
        Backtest {
            data,
            cash,
            broker,
            strategy,
            commission,
            bidask_spread,
            margin,
            trade_on_close,
            hedging,
            exclusive_orders,
        }
    }
    
    // run the simulation over all ticks in the provided data.
    pub fn run(&mut self) {
        use indicatif::{ProgressBar, ProgressStyle};

        self.strategy.init(&mut self.broker, &self.data);
        
        let n = self.data.close.len();
        
        let pb = ProgressBar::new(n as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{desc:.green} {bar:40.white} {percentage:>3}% | {pos:>7}/{len:7} [{elapsed_precise}<{eta_precise}] {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  ")); 

        pb.set_message("Running backtest...");
        
        for index in 0..n {
            self.broker.next(index);
            self.strategy.next(&mut self.broker, index);
            pb.set_position(index as u64);
        }
        pb.finish_with_message("");

        // print stats after backtest completes
        self.broker.print_trading_stats();
        // save trade log to file instead of printing to console
        if let Err(e) = self.broker.save_trade_log("output_trade_log.txt") {
            println!("error saving trade log: {:?}", e);
        } else {
            println!("trade log successfully saved to trade_log.txt");
        }
    }

    // abstraction for plotting the equity curve
    // this method converts date strings to NaiveDateTime, pairs them with equity values,
    // and calls the plot_equity function to generate the plot.
    pub fn plot(&self, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        
        let equity_history: Vec<(NaiveDateTime, f64)> = self.data.date.iter()
            .zip(self.broker.equity.iter())
            .map(|(date_str, &equity)| {
                // adjust the format string to match your data; for example: "2020-01-01 23:01:00"
                let dt = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                    .expect("failed to parse date");
                (dt, equity)
            })
            .collect();

        // call the external plotting function from plot.rs
        plot_equity(&equity_history, output_path)
    }

    pub fn plot_equity_and_benchmark(&self, benchmark: &Vec<f64>, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // convert to percentage changes from initial values
        let initial_equity = self.broker.equity[0];
        let initial_benchmark = benchmark[0];

        let equity_history: Vec<(NaiveDateTime, f64)> = self.data.date.iter()
            .zip(self.broker.equity.iter())
            .map(|(date_str, &equity)| {
                let dt = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                    .expect("failed to parse date");
                let pct_change = (equity - initial_equity) / initial_equity * 100.0;
                (dt, pct_change)
            })
            .collect();

        let benchmark_history: Vec<(NaiveDateTime, f64)> = self.data.date.iter()
            .zip(benchmark.iter())
            .map(|(date_str, &value)| {
                let dt = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                    .expect("failed to parse date");
                let pct_change = (value - initial_benchmark) / initial_benchmark * 100.0;
                (dt, pct_change)
            })
            .collect();

        plot_equity_and_benchmark(&equity_history, &benchmark_history,output_path)
    }

    pub fn plot_margin_usage(&self, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let margin_usage_history: Vec<(NaiveDateTime, f64)> = self.data.date.iter()
            .zip(self.broker.margin_usage_history.iter())
            .map(|(date_str, &margin_usage)| {
                let dt = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                    .expect("failed to parse date");
                (dt, margin_usage)
            })
            .collect();

        plot_margin_usage(&margin_usage_history, output_path)
    }
    
} 