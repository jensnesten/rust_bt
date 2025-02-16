use tch::{CModule, Tensor};
use anyhow::Result;

pub struct NeuralNet {
    model: CModule,
}

impl NeuralNet {
    // load the TorchScript model from file
    pub fn new(model_path: &str) -> Result<Self> {
        let model = CModule::load(model_path)?;
        Ok(Self { model })
    }

    // run a forward pass given a slice of input data (adjust dimensions as needed)
    pub fn predict(&self, input: &[f32]) -> Result<Tensor> {
        // create a tensor from input data and add a batch dimension (unsqueeze)
        let input_tensor = Tensor::of_slice(input).reshape(&[1, 4]);
        let output = self.model.forward_ts(&[input_tensor])?;
        Ok(output)
    }
}

