use rust_live::stream::single;
use rust_core::live_strategies::live_statarbspread::LiveStatArbSpreadStrategy;
use rust_core::engine::{LiveBroker, LiveBacktest};

// main entry point bringing live trading engine and strategy together
#[tokio::main]
async fn main() {
    // print startup message
    println!("starting live trading engine...");

    // fetch live data using the stream module (returns a vector of live data messages)
    let live_data_vec = single().await;

    // check that we received at least one message
    if live_data_vec.is_empty() {
        println!("no live data received, exiting...");
        return;
    }

    // use the first live data message as our initial data
    let initial_live_data = live_data_vec[0].clone();

    // create an instance of the live stat arb spread strategy
    let strategy =
        Box::new(rust_core::live_strategies::live_statarbspread::LiveStatArbSpreadStrategy::new())
            as rust_core::engine::LiveStrategyRef;

    // build a live backtest instance with chosen parameters
    let mut live_backtest = rust_core::engine::LiveBacktest::new(
        initial_live_data,
        strategy,
        100000.0,  // live_cash
        0.0001,    // live_bidask_spread
        1.0,       // live_margin (no leverage)
        false,     // live_trade_on_close
        false,     // live_hedging
        false,     // live_exclusive_orders
        false,     // live_scaling_enabled
    );

    // run the live trading engine (this will iterate over tick indices in the live data)
    live_backtest.run();

    live_backtest.broker.print_live_stats(live_backtest.data.ask.len() - 1);

    // print the live backtest stats
}
    