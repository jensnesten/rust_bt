import joblib
import json

# load the scaler (if not already loaded)
scaler = joblib.load('../inference/models/imp_scaler.pkl')

# export the scaler parameters to JSON
scaler_params = {
    "mean": scaler.mean_.tolist(),
    "scale": scaler.scale_.tolist()
}
with open("../inference/models/scaler_params.json", "w") as f:
    json.dump(scaler_params, f)