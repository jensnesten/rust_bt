use tch::{CModule, Tensor};
use anyhow::Result;

pub struct NeuralNet {
    model: CModule,
}

impl NeuralNet {
    // load the TorchScript model from file
    pub fn new(rel_path: &str) -> Result<Self, tch::TchError> {
        // build path relative to rust_ml's manifest
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = base.join(rel_path);
        // load torchscript model from the resolved path
        let model = tch::CModule::load(model_path)?;
        Ok(NeuralNet { model })
    }

    // run a forward pass given a slice of input data (adjust dimensions as needed)
    pub fn predict(&self, input: &[f32]) -> Result<Tensor> {
        // create a tensor from input data and add a batch dimension (unsqueeze)
        let input_tensor = Tensor::from(input).reshape(&[1, 4]);
        let output = self.model.forward_ts(&[input_tensor])?;
        Ok(output)
    }
}

