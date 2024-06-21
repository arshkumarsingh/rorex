
# Forex Rate Fetcher

A Rust application that fetches and displays forex rates using a graphical user interface (GUI). The application also fetches historical forex rates and plots the data on a chart.

## Features

- Fetch current forex rates
- Fetch historical forex rates for the past 30 days
- Display forex rates and trends on a chart
- Select base and target currencies from a dropdown menu
- Enter your own API key

## Dependencies

This project uses the following Rust crates:

- `reqwest` for making HTTP requests
- `serde` and `serde_json` for JSON parsing
- `eframe` and `egui` for the GUI
- `egui_plot` for plotting data
- `chrono` for date and time handling

## Installation

1. Ensure you have Rust and Cargo installed. If not, follow the instructions [here](https://www.rust-lang.org/tools/install).

2. Clone the repository:

   ```sh
   git clone https://github.com/yourusername/forex_gui.git
   cd forex_gui
   ```

3. Add your API key in the application when prompted or modify the code directly with your API key.

4. Build and run the application:

   ```sh
   cargo run
   ```

## Usage

1. Enter your API key in the provided text box. You can obtain an API key from [ExchangeRate-API](https://www.exchangerate-api.com/).

2. Select the base and target currencies from the dropdown menus.

3. Click "Fetch Rate" to get the current forex rate.

4. Click "Fetch Historical Rates" to get the historical forex rates for the past 30 days.

5. View the current and historical rates plotted on the chart.

## Code Overview

The main parts of the application are:

- `main.rs`: The entry point of the application. It sets up the GUI, handles user interactions, and fetches data from the API.
- `fetch_forex_rate`: Fetches the current forex rate.
- `fetch_historical_rates`: Fetches historical forex rates for the past 30 days.

## Contributing

Feel free to open issues or submit pull requests for improvements and new features.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
