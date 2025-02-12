use reqwest::Error;
use serde_json::Value;
use std::env;
use dotenv::dotenv;

const FRED_TBILL_3M_URL: &str = "https://api.stlouisfed.org/fred/series/observations?series_id=TB3MS&api_key={API_KEY}&file_type=json";

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok(); // Load .env file
    let api_key = env::var("FRED_API_KEY").expect("FRED_API_KEY not set in .env");
    
    let url = FRED_TBILL_3M_URL.replace("{API_KEY}", &api_key);
    
    let response = reqwest::get(&url).await?;
    let json: Value = response.json().await?;
    
    if let Some(observations) = json["observations"].as_array() {
        if let Some(latest) = observations.last() {
            if let Some(rate) = latest["value"].as_str() {
                println!("Latest 3-Month T-Bill Rate: {}%", rate);
            }
        }
    }
    
    Ok(())
}
