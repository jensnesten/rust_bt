use rust_core::engine::{Backtest, Strategy};
use rust_core::stats::compute_stats;
#[allow(unused_imports)]
use rust_core::strategies::statarb_spread::StatArbSpreadStrategy;
#[allow(unused_imports)]
use rust_core::strategies::sma::SmaStrategy;
#[allow(unused_imports)]
use rust_core::strategies::simple_strategy::SimpleStrategy;
#[allow(unused_imports)]
use rust_core::strategies::statarb_pairs::StatArbPairsStrategy;
#[allow(unused_imports)]
use rust_core::strategies::scaled_statarb_pairs::ScaledStatArbPairsStrategy;
#[allow(unused_imports)]
use rust_core::strategies::dynamic_pairs::DynamicPairsStrategy;
#[allow(unused_imports)]
use rust_core::strategies::ml_statarb_pairs::MLStatArbPairsStrategy;
use rust_core::data_handler::handle_ohlc;
use std::time::Instant;

fn main() {
    //start time
    let start = Instant::now();

    let data = handle_ohlc("/Users/jarlen/NHNTrading/rust_bt/rust_bt/data/SP500_DJIA_fyear_clean.csv").expect("Failed to load CSV data");

    let cash = 100_000.0;
    let commission = 0.0;
    let bidask_spread = 0.0;
    let margin = 0.05;
    let trade_on_close = false;
    let hedging = false;
    let exclusive_orders = false;
    let scaling_enabled = true;

    // boxed instance of strategy
    let strategy: Box<dyn Strategy> = Box::new(ScaledStatArbPairsStrategy::new());

    let mut backtest = Backtest::new(
        data,
        strategy,
        cash,
        commission,
        bidask_spread,
        margin,
        trade_on_close,
        hedging,
        exclusive_orders,
        scaling_enabled, // enable scaling
    );

    backtest.run();

    let stats = compute_stats(
        &backtest.broker.closed_trades,
        &backtest.broker.equity,
        &backtest.data,
        0.0421, // risk free rate as fraction
        backtest.broker.max_margin_usage // pass max margin usage
    );

    println!("{}", stats);
    println!("time taken: {:?}", start.elapsed());
    
    if let Err(e) = backtest.plot_equity_and_benchmark(&backtest.data.close, "output_equity.png") {
        eprintln!("error generating plot: {}", e);
    }

    if let Err(e) = backtest.plot_margin_usage("output_margin_usage.png") {
        eprintln!("error generating plot: {}", e);
    }
} 