use rust_live::stream::single;

fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(single());
}
    