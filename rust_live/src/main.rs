use tokio::sync::mpsc;
use rust_live::stream::stream_live_data;
use rust_core::engine::{LiveBacktest, LiveData, LiveStrategyRef};
use rust_core::strategies::live_statarb_spread::LiveStatArbSpreadStrategy;

#[tokio::main]
async fn main() {
    // print startup message
    println!("starting live trading engine...");

    // create an unbounded channel for live data
    let (tx, mut rx) = mpsc::unbounded_channel::<LiveData>();

    // spawn the streaming task; it continuously sends live data via the channel
    tokio::spawn(async move {
        stream_live_data(tx).await;
    });

    // wait for the first live data message to initialize the simulation
    let initial_live_data = rx.recv().await.expect("no live data received");

    // create a boxed live strategy instance
    let strategy: LiveStrategyRef = Box::new(LiveStatArbSpreadStrategy::new());

    // build the live backtest instance with the initial live data
    let mut live_backtest = LiveBacktest::new(
        initial_live_data.clone(),
        strategy,
        100_000.0,  // live_cash
        1.0,        // live_margin (no leverage)
        false,      // live_trade_on_close
        false,      // live_hedging
        false,      // live_exclusive_orders
        false,      // live_scaling_enabled
    );

    // include the initial data in broker as well
    live_backtest.broker.live_data = initial_live_data;

    // run the live simulation driven by new live data messages
    live_backtest.run(rx).await;
}