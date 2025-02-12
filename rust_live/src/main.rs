use rust_live::stream::pairs;

fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(pairs());
}
    