import torch
import torch.nn as nn
import pandas as pd
import torch.optim as optim
from sklearn.model_selection import train_test_split
from sklearn.preprocessing import StandardScaler

data = pd.read_csv('../Data/SP500_DJIA_2020_clean.csv', index_col=0, parse_dates=True)

device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
#add gpu cuda if available
#device = torch.device("cuda" if torch.cuda.is_available() else device)
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

# Convert to PyTorch tensors
X_train = torch.tensor(X_train, dtype=torch.float32).to(device)
y_train = torch.tensor(y_train, dtype=torch.long).to(device)
X_test = torch.tensor(X_test, dtype=torch.float32).to(device)
y_test = torch.tensor(y_test, dtype=torch.long).to(device)

class DeepNN(nn.Module):
    def __init__(self):
        super(DeepNN, self).__init__()
        self.fc1 = nn.Linear(4, 512)
        self.bn1 = nn.BatchNorm1d(512)
        self.fc2 = nn.Linear(512, 512)
        self.bn2 = nn.BatchNorm1d(512)
        self.fc3 = nn.Linear(512, 256)
        self.bn3 = nn.BatchNorm1d(256)
        self.fc4 = nn.Linear(256, 128)
        self.bn4 = nn.BatchNorm1d(128)
        self.fc5 = nn.Linear(128, 3)
        self.dropout = nn.Dropout(0.5)

    def forward(self, x):
        x = torch.relu(self.bn1(self.fc1(x)))
        x = self.dropout(x)
        x = torch.relu(self.bn2(self.fc2(x)))
        x = self.dropout(x)
        x = torch.relu(self.bn3(self.fc3(x)))
        x = self.dropout(x)
        x = torch.relu(self.bn4(self.fc4(x)))
        x = self.dropout(x)
        x = self.fc5(x)
        return x

# Initialize the model, loss function, and optimizer
model = DeepNN().to(device)
criterion = nn.CrossEntropyLoss()
optimizer = optim.Adam(model.parameters(), lr=0.00658)

# Train the model
num_epochs = 200
for epoch in range(num_epochs):
    model.train()
    optimizer.zero_grad()
    outputs = model(X_train)
    loss = criterion(outputs, y_train)
    loss.backward()
    optimizer.step()

    if (epoch+1) % 10 == 0:
        print(f'Epoch [{epoch+1}/{num_epochs}], Loss: {loss.item():.4f}')

# Evaluate the model
model.eval()
with torch.no_grad():
    outputs = model(X_test)
    _, predicted = torch.max(outputs.data, 1)
    accuracy = (predicted == y_test).sum().item() / y_test.size(0)
    print(f'Accuracy: {accuracy:.4f}')

torch.save(model.state_dict(), 'imp_model.pth')

import joblib
# Save the scaler
joblib.dump(scaler, 'imp_scaler.pkl')
