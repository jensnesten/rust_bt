use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

// owercase: struct to hold scaler parameters
#[derive(Deserialize)]
pub struct ScalerParams {
    pub mean: Vec<f32>,
    pub scale: Vec<f32>,
}

// owercase: load scaler params from a json file
pub fn load_scaler_params(path: &str) -> ScalerParams {
    let file = File::open(path).expect("failed to open scaler params file");
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).expect("failed to read scaler params")
}

// owercase: apply scaling to the input vector
pub fn scale_input(input: &[f32], params: &ScalerParams) -> Vec<f32> {
    input.iter()
        .enumerate()
        .map(|(i, &x)| (x - params.mean[i]) / params.scale[i])
        .collect()
}