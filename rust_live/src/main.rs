use tokio::sync::mpsc;
use rust_live::stream::stream_live_data;
use rust_live::stream::pairs;
use rust_live::stream::stream_live_data_pairs;
use rust_core::live_engine::{LiveBacktest, LiveData, LiveStrategyRef};
use rust_core::strategies::live_statarb_spread::LiveStatArbSpreadStrategy;
use rust_core::strategies::live_statarb_pairs::LiveStatArbPairsStrategy;
//use rust_core::strategies::live_ml_statarb_spread::LiveMLStatArbSpreadStrategy;

#[tokio::main]
async fn main() {
    // print startup message
    println!("starting live testing engine...");

    // create a channel for live data
    let (tx, mut rx) = mpsc::unbounded_channel::<LiveData>();

    let reference_id1 = "US500";
    let uic1 = 4913;
    let reference_id2 = "DJIA";
    let uic2 = 4911;

    // spawn streaming task for instrument 1
    tokio::spawn({
        let tx1 = tx.clone();
        async move {
            pairs(tx1, reference_id1, uic1, reference_id2, uic2).await;
        }
    });

    // wait for initial data from both streams (customize as needed)
    let initial_data1 = rx.recv().await.expect("no live data from instrument 1");

    // create a live strategy (example using the pairs strategy)
    let strategy: LiveStrategyRef = Box::new(LiveStatArbPairsStrategy::new());

    // initialize live backtest with one of the initial messages, or merge the two
    let mut live_backtest = LiveBacktest::new(
        initial_data1.clone(), // or a combined data structure if needed
        strategy,
        100_000.0,  // live cash
        0.05,       // live margin
        false,      // trade on close
        false,      // hedging
        false,      // exclusive orders
        false,      // scaling enabled
    );
    
    // optionally set the second stream data
    live_backtest.broker.live_data = initial_data1;
    
    // run the simulation consuming all incoming live data
    live_backtest.run(rx).await;
}