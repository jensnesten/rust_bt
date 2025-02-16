#%%
import torch
from export_model import DeepNN  

model = DeepNN()
model.load_state_dict(torch.load("../models/model.pth"))
model.eval()

example_input = torch.randn(1, 4)
traced_script_module = torch.jit.trace(model, example_input)
traced_script_module.save("model.pt")

# %%
