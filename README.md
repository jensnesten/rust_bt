# rust_bt 🦀

A high performance, low-latency backtesting engine for testing quantitative trading strategies in Rust. For backtesting on historical data OHLC data is required, and for live trading bid/ask data is required.

The engine is designed to be used in conjunction with a strategy that implements the `Strategy` trait. The strategy is responsible for making trading decisions based on the `Broker` struct. 

It's barebones by design, and is intended to be expanded upon to align with the relevant market microstructure and fit the type of products you intend to trade.  

## Overview

- High performance, low-latency 
- Flexible, modular design
- Complete tick-by-tick market simulation
- Complete live trading engine 
- Market microstructure simulation, including bid-ask spread, slippage, commissions, etc.
- Detailed trade and position management, fractional orders
- Contingent orders (SL/TP), Auto-scaling methods
- Margin and leverage management for complex instruments
- Pairs trading, Trading multiple instruments
- Plotting and statistics

## Project Components

- **rust_core**: the central trading engine  
  *// houses strategies, orderbook logic, and data handling; the core intelligence behind the system*

- **rust_live**: the live trading interface  
  *// connects the core trading logic to real-time data and execution; for live trading, navigate to the `rust_live` directory and run `cargo run`*

- **rust_bt**: the backtesting interface  
  *// integrates the core trading functionality with historical data for simulation; for backtesting, navigate to the `rust_bt` directory and run `cargo run`*

### How It Works

the strategies are implemented in **rust_core**, but they are adapted to suit different operational environments:

- **Backtesting Strategies**  
  *// built for historical data simulation*  
  backtesting strategies use the standard engine types such as `Broker`, `OhlcData`, `Order`, and `Strategy`.
  these types are designed to work with preloaded historical market data, allowing the simulation of trades over past time periods. the backtesting engine in **rust_bt** orchestrates the process, ensuring that trades are simulated in a controlled, time-sequential manner.

- **Live Trading Strategies**  
  *// built for processing real-time market data*  
  live strategies are implemented with dedicated live engine types like `LiveBroker`, `LiveData`, `Order`, and `LiveStrategy`.  
  These types are specifically designed to handle streaming market data and execute orders as market conditions evolve, ensuring that order placement, execution, and statistics (like pnl) update in real time.

this design ensures that while the core trading logic remains consistent in **rust_core**, each operational mode (backtest or live) uses the appropriate interface to manage data, process orders, and update trade statistics optimally. For examples see the `rust_core/src/strategies/statarb_pairs.rs` and `rust_core/src/strategies/live_statarb_spread.rs` strategies.

## Backtesting 

Strategies are implemented by creating a new struct in `rust_core/src/strategies/` that implements the `Strategy` trait:

```rust
use crate::engine::{Broker, OhlcData, Order, Strategy};
pub struct MyStrategy;

impl Strategy for MyStrategy {
    fn init(&mut self, broker: &mut Broker, data: &OhlcData) {
        // initialization can precompute indicators, etc..
    }

    fn next(&mut self, broker: &mut Broker, index: usize) {
        // implement the strategy logic here
    }
}
```

### Opening a position
The `Broker` struct provides the following core functionality:

- `new_order(order: Order)`: Places a new order
- `closed_trades(trade: Trade)`: Closes a trade
- `close_all_trades()`: Closes all trades
- `cash += closed_trade.pnl()`: Updates the cash balance

Orders are processed on every tick, and the `next` method is called on every tick.

To create a buy order we need to specify the size, and optionally the stop loss, take profit, limit, parent trade and instrument (to trade multiple instruments, default is 1).

```rust
let order = Order {
    size: trade.size,
    sl: None,
    tp: None,
    limit: None,
    stop: None,
    parent_trade: None,
    instrument: 1,
};
broker.new_order(order);
self.positions.register_position(trade.size); // track order with PositionManager (optional)
```
### PositionManager
The `PositionManager` provides a simple interface for handling all types of positions:

```rust
use crate::position::PositionManager;
let mut positions = PositionManager::new(3); // allow max 3 positions per side (Long and Short)
positions.register_position(trade.size); // register a long position
positions.register_position(-trade.size); // register a short position
positions.close_position(trade.size); // register closing a long position
positions.close_position(-trade.size); // register closing a short position
```
The `PositionManager` doesnt open or close positions, it simply tracks them in parallel for more granular control. This allows for more complex order management, which then enables us to implement more sophisticated hedging techniques in real-time. 

### Closing a position
To close a position we use the `Trade` struct. After closing each trade we need to update the cash balance and add the trade to the closed trades vector - alongside with updating the position manager if used:

```rust
let trade = broker.trades.remove(0); //closes first position in trades vector
let closed_trade = Trade {
    size: trade.size,
    entry_price: trade.entry_price,
    entry_index: trade.entry_index,
    exit_price: Some(price),
    exit_index: Some(index),
    sl_order: trade.sl_order,
    tp_order: trade.tp_order,
};
broker.cash += closed_trade.pnl();
broker.closed_trades.push(closed_trade);
self.positions.close_position(trade.size);
```

To close all positions we need to delete each element in the `trades` vector and update our stats accordingly. We do this by calling the `close_all_trades` method from the `Broker` struct.

### Plotting

The `backtest.plot()` function is used to plot the equity curve. It takes a slice of (naivedatetime, equity_value) tuples and an output file path.

```rust
if let Err(e) = backtest.plot("output_equity_plot.png") {
    eprintln!("error generating plot: {}", e);
}
```

## Live Trading 

Strategies are implemented in the same way as for backtesting, but the `next` method is called on every tick of the live data, where every 'tick' is a data event. Here we use the LiveStrategy trait:

```rust
use crate::live_engine::{LiveBroker, LiveData, Order, LiveStrategy};
pub struct MyStrategy;

impl LiveStrategy for MyLiveStrategy {
    fn init(&mut self, broker: &mut LiveBroker, data: &LiveData) {
        // initialization can precompute indicators, etc..
    }

    fn next(&mut self, broker: &mut LiveBroker, index: usize) {
        // implement the strategy logic here
    }
}
```

### 






