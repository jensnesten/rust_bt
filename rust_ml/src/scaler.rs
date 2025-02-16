use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

//  struct to hold scaler parameters
#[derive(Deserialize)]
pub struct ScalerParams {
    pub mean: Vec<f32>,
    pub scale: Vec<f32>,
}

// load scaler params from a json file
pub fn load_scaler_params(rel_path: &str) -> ScalerParams {
    // base from rust_ml cargo manifest
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let full_path = base.join(rel_path); // join with passed relative path
    let file = File::open(&full_path).expect("failed to open scaler params file");
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).expect("failed to read scaler params")
}

// apply scaling to the input vector
pub fn scale_input(input: &[f32], params: &ScalerParams) -> Vec<f32> {
    input.iter()
        .enumerate()
        .map(|(i, &x)| (x - params.mean[i]) / params.scale[i])
        .collect()
}