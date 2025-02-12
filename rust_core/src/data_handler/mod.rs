use csv::ReaderBuilder;
use std::error::Error;
use crate::engine::OhlcData;

// data handler for simple csv
pub fn load_csv(path: &str) -> Result<OhlcData, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)?;

    let mut date = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut close2 = Vec::new();
    
    for result in rdr.records() {
        let record = result?;
        date.push(record[0].to_string());
        open.push(record[1].parse::<f64>()?);
        high.push(record[2].parse::<f64>()?);
        low.push(record[3].parse::<f64>()?);
        close.push(record[4].parse::<f64>()?);
        let close2_val = if record[5].trim().is_empty() {
            0.0
        } else {
            record[5].parse::<f64>()?
        };
        close2.push(close2_val);
    }
    
    Ok(OhlcData {
        date,
        open,
        high,
        low,
        close,
        close2,
        volume: None,
    })
}