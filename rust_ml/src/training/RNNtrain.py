import pandas as pd
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from sklearn.model_selection import train_test_split
from sklearn.preprocessing import StandardScaler

# Load and preprocess data
data = pd.read_csv('../Data/SP500_DJIA_2m_clean.csv', index_col=0, parse_dates=True)

device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
print(f"Using device: {device}")

# Create features
data['Spread'] = data['Close'] / data['Close'].shift(2) - data['Close2'] / data['Close2'].shift(2)
data['Spread_Mean'] = data['Spread'].rolling(window=20).mean()
data['Spread_Std'] = data['Spread'].rolling(window=20).std()
data['Zscore'] = (data['Spread'] - data['Spread_Mean']) / data['Spread_Std']

data.dropna(inplace=True)

# Create target variable
data['Signal'] = 0
data.loc[data['Zscore'] > 1.0, 'Signal'] = 2  # Sell signal (mapped from -1 to 2)
data.loc[data['Zscore'] < -1.0, 'Signal'] = 0  # Buy signal (mapped from 1 to 0)
data.loc[(data['Zscore'] <= 1.0) & (data['Zscore'] >= -1.0), 'Signal'] = 1  # Hold signal (mapped from 0 to 1)

# Features and target
features = ['Spread', 'Spread_Mean', 'Spread_Std', 'Zscore']
X = data[features].values
y = data['Signal'].values

# Split data into training and testing sets
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2, random_state=42)

# Standardize the features
scaler = StandardScaler()
X_train = scaler.fit_transform(X_train)
X_test = scaler.transform(X_test)

# Convert to sequences
def create_sequences(data, target, seq_length):
    xs = []
    ys = []
    for i in range(len(data) - seq_length):
        x = data[i:i+seq_length]
        y = target[i+seq_length]
        xs.append(x)
        ys.append(y)
    return np.array(xs), np.array(ys)

seq_length = 20
X_train_seq, y_train_seq = create_sequences(X_train, y_train, seq_length)
X_test_seq, y_test_seq = create_sequences(X_test, y_test, seq_length)

# Convert to PyTorch tensors
X_train_seq = torch.tensor(X_train_seq, dtype=torch.float32).to(device)
y_train_seq = torch.tensor(y_train_seq, dtype=torch.long).to(device)
X_test_seq = torch.tensor(X_test_seq, dtype=torch.float32).to(device)
y_test_seq = torch.tensor(y_test_seq, dtype=torch.long).to(device)

# Define the LSTM model
class LSTMModel(nn.Module):
    def __init__(self, input_size, hidden_size, num_layers, output_size):
        super(LSTMModel, self).__init__()
        self.hidden_size = hidden_size
        self.num_layers = num_layers
        self.lstm = nn.LSTM(input_size, hidden_size, num_layers, batch_first=True)
        self.fc = nn.Linear(hidden_size, output_size)
        self.dropout = nn.Dropout(0.5)

    def forward(self, x):
        h0 = torch.zeros(self.num_layers, x.size(0), self.hidden_size).to(x.device)
        c0 = torch.zeros(self.num_layers, x.size(0), self.hidden_size).to(x.device)
        out, _ = self.lstm(x, (h0, c0))
        out = self.dropout(out[:, -1, :])
        out = self.fc(out)
        return out

# Initialize the model, loss function, and optimizer
model = LSTMModel(input_size=4, hidden_size=128, num_layers=4, output_size=3).to(device)
criterion = nn.CrossEntropyLoss()
optimizer = optim.Adam(model.parameters(), lr=0.001)  # Adjusted learning rate

# Train the model
num_epochs = 100  # Adjusted number of epochs
train_losses = []
test_losses = []
for epoch in range(num_epochs):
    model.train()
    optimizer.zero_grad()
    outputs = model(X_train_seq)
    loss = criterion(outputs, y_train_seq)
    loss.backward()
    optimizer.step()
    train_losses.append(loss.item())

    # Evaluate on test set
    model.eval()
    with torch.no_grad():
        test_outputs = model(X_test_seq)
        test_loss = criterion(test_outputs, y_test_seq)
        test_losses.append(test_loss.item())

    if (epoch+1) % 1 == 0:
        print(f'Epoch [{epoch+1}/{num_epochs}], Train Loss: {loss.item():.4f}, Test Loss: {test_loss.item():.4f}')

# Evaluate the model
model.eval()
with torch.no_grad():
    outputs = model(X_test_seq)
    _, predicted = torch.max(outputs.data, 1)
    accuracy = (predicted == y_test_seq).sum().item() / y_test_seq.size(0)
    print(f'Accuracy: {accuracy:.4f}')

# Save the model and scaler
torch.save(model.state_dict(), 'rnn_model.pth')
import joblib
joblib.dump(scaler, 'rnn_scaler.pkl')

# Plot the training and test loss
import matplotlib.pyplot as plt
plt.plot(train_losses, label='Train Loss')
plt.plot(test_losses, label='Test Loss')
plt.xlabel('Epoch')
plt.ylabel('Loss')
plt.legend()
plt.show()
