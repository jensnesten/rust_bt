
# rust_bt 🦀

A high performance, low-latency backtesting engine for testing quantitative trading strategies in Rust. Currently under construction - live testing coming soon....

The engine is designed to be used in conjunction with a strategy that implements the `Strategy` trait. The strategy is responsible for making trading decisions based on the `Broker` struct. 

It's barebones by design, and is intended to be expanded upon to align with the relevant market microstructure and fit the type of products you intend to trade.  

## Key Features

- High performance, low-latency 
- Flexible, modular design
- Complete tick-by-tick market simulation
- Market microstructure simulation, including bid-ask spread, slippage, commissions, etc.
- Detailed trade and position management, fractional orders
- Contingent orders (SL/TP), Auto-scaling methods
- Margin and leverage management for complex instruments
- Pairs trading, Trading multiple instruments
- Plotting and statistics

## Usage

Strategies are implemented by creating a new struct in `rust_core/src/strategies/` that implements the `Strategy` trait:

```rust
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
- `cash += closed_trade.pnl()`: Updates the cash balance

Orders are processed on every tick, and the `next` method is called on every tick.

To create a buy order we need to specify the size, and optionally the stop loss, take profit, limit and parent trade.

```rust
let order = Order {
    size: trade.size,
    sl: None,
    tp: None,
    limit: None,
    stop: None,
    parent_trade: None,
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







