use chrono::{NaiveDate, Utc};
use eframe::egui;
use egui::{CentralPanel, ComboBox};
use egui_plot::{Line, Plot, PlotPoints};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

#[derive(Deserialize)]
struct ForexResponse {
    conversion_rates: HashMap<String, f64>,
}

#[derive(Deserialize)]
struct HistoricalResponse {
    rates: HashMap<String, HashMap<String, f64>>,
}

/// Fetches the forex rate for a given currency pair using the provided API key.
///
/// # Arguments
///
/// * `api_key` - The API key for the ExchangeRate-API service.
/// * `pair` - The currency pair to fetch the rate for. Format: "base_currencytarget_currency".
///
/// # Returns
///
/// * `Result<f64, String>` - The forex rate if successful, otherwise an error message.
///
/// # Errors
///
/// If the HTTP request fails or the JSON deserialization fails, an error message is returned.
/// If the currency pair is not found in the response, an error message is returned.
pub fn fetch_forex_rate(api_key: &str, pair: &str) -> Result<f64, String> {
    // Construct the URL for the API request
    let url = format!("https://v6.exchangerate-api.com/v6/{}/latest/{}", api_key, &pair[..3]);

    // Create a new HTTP client
    let client = Client::new();

    // Send a GET request to the API and get the response
    let response = client.get(&url)
        .send()
        .map_err(|e| e.to_string())?;

    // Deserialize the API response into a struct
    let forex_response: ForexResponse = response
        .json()
        .map_err(|e| e.to_string())?;

    // Get the target currency rate from the response
    let target_currency = &pair[3..];
    forex_response.conversion_rates.get(target_currency)
        .copied()
        .ok_or_else(|| {
            "Currency pair not found".to_string()
        })
}

fn fetch_historical_rates(api_key: &str, pair: &str) -> Result<Vec<(NaiveDate, f64)>, String> {
    let base_currency: &str = &pair[..3];
    let target_currency: &str = &pair[3..];
    let mut rates: Vec<(NaiveDate, f64)> = Vec::new();
    let end_date: NaiveDate = Utc::now().date_naive();
    let start_date: NaiveDate = end_date - chrono::Duration::days(30);

    for date in (0..=30).map(|i| start_date + chrono::Duration::days(i)) {
        let url = format!(
            "https://v6.exchangerate-api.com/v6/{}/history/{}/{}?start_date={}&end_date={}",
            api_key, base_currency, target_currency, start_date, end_date
        );

        let client = Client::new();
        let response = client.get(&url).send().map_err(|e| e.to_string())?;
        let historical_response: HistoricalResponse = response.json().map_err(|e| e.to_string())?;

        if let Some(rate) = historical_response.rates.get(&date.to_string()) {
            if let Some(&rate) = rate.get(target_currency) {
                rates.push((date, rate));
            }
        }
    }

    Ok(rates)
}

/// Represents the application state.
struct App {
    /// The API key for the ExchangeRate-API service.
    api_key: String,
    /// The base currency for forex conversion.
    base_currency: String,
    /// The target currency for forex conversion.
    target_currency: String,
    /// The forex rate if it has been fetched.
    rate: Option<f64>,
    /// The sender end of a channel for fetching the forex rate.
    fetch_rate_tx: Sender<Option<f64>>,
    /// The receiver end of a channel for fetching the forex rate.
    fetch_rate_rx: Receiver<Option<f64>>,
    /// The list of available currencies.
    currencies: Vec<&'static str>,
    /// The trend of historical rates.
    ///
    /// The trend is represented as a vector of `f64` values, where each value
    /// represents the rate on a specific date. The dates are not explicitly
    /// stored in the vector, but can be inferred from the vector index.
    trend: Vec<f64>,
    /// The historical rates for a given currency pair.
    ///
    /// The historical rates are represented as a vector of tuples, where each
    /// tuple contains the date and the rate on that date.
    historical_rates: Vec<(NaiveDate, f64)>,
}
/// Represents the application state.
impl App {
    /// Creates a new instance of `App`.
    ///
    /// Returns an instance of `App` with default values for all fields.
    pub fn new() -> Self {
        // Create a channel for fetching the forex rate
        let (fetch_rate_tx, fetch_rate_rx) = mpsc::channel();
        
        App {
            // API key for the ExchangeRate-API service
            api_key: "".to_string(),
            // Base currency for forex conversion
            base_currency: "USD".to_string(),
            // Target currency for forex conversion
            target_currency: "EUR".to_string(),
            // Fetched forex rate
            rate: None,
            // Sender end of a channel for fetching the forex rate
            fetch_rate_tx,
            // Receiver end of a channel for fetching the forex rate
            fetch_rate_rx,
            // List of available currencies
            currencies: vec![
                // Currencies are listed in alphabetical order
                "AED", "AFN", "ALL", "AMD", "ANG", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT",
                "BGN", "BHD", "BIF", "BMD", "BND", "BOB", "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CAD",
                "CHF", "CLP", "CNY", "COP", "CRC", "CUP", "CVE", "CZK", "DJF", "DKK", "DOP", "DZD", "EGP",
                "ERN", "ETB", "EUR", "FJD", "FKP", "FOK", "GBP", "GEL", "GGP", "GHS", "GIP", "GMD", "GNF",
                "GTQ", "GYD", "HKD", "HNL", "HRK", "HTG", "HUF", "IDR", "ILS", "IMP", "INR", "IQD", "IRR",
                "ISK", "JEP", "JMD", "JOD", "JPY", "KES", "KGS", "KHR", "KID", "KMF", "KPW", "KRW", "KWD",
                "KYD", "KZT", "LAK", "LBP", "LKR", "LRD", "LSL", "LYD", "MAD", "MDL", "MGA", "MKD", "MMK",
                "MNT", "MOP", "MRU", "MUR", "MVR", "MWK", "MXN", "MYR", "MZN", "NAD", "NGN", "NIO", "NOK",
                "NPR", "NZD", "OMR", "PAB", "PEN", "PGK", "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD",
                "RUB", "RWF", "SAR", "SBD", "SCR", "SDG", "SEK", "SGD", "SHP", "SLL", "SOS", "SRD", "SSP",
                "STN", "SYP", "SZL", "THB", "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TVD", "TWD", "TZS",
                "UAH", "UGX", "UYU", "UZS", "VES", "VND", "VUV", "WST", "XAF", "XCD", "XDR", "XOF", "XPF",
                "YER", "ZAR", "ZMW", "ZWL",
            ],
            // Trend of historical rates
            // The trend is represented as a vector of `f64` values, where each value
            // represents the rate on a specific date. The dates are not explicitly
            // stored in the vector, but can be inferred from the vector index.
            trend: Vec::new(),
            // Historical rates for a given currency pair
            // The historical rates are represented as a vector of tuples, where each
            // tuple contains the date and the rate on that date.
            historical_rates: Vec::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Forex Rate Fetcher");

            ui.horizontal(|ui| {
                ui.label("API Key:");
                ui.text_edit_singleline(&mut self.api_key);
            });

            ui.horizontal(|ui| {
                ui.label("Base Currency:");
                ComboBox::from_id_source("base_currency")
                    .selected_text(&self.base_currency)
                    .show_ui(ui, |ui| {
                        for currency in &self.currencies {
                            ui.selectable_value(&mut self.base_currency, currency.to_string(), currency.to_string());
                        }
                    });

                ui.label("Target Currency:");
                ComboBox::from_id_source("target_currency")
                    .selected_text(&self.target_currency)
                    .show_ui(ui, |ui| {
                        for currency in &self.currencies {
                            ui.selectable_value(&mut self.target_currency, currency.to_string(), currency.to_string());
                        }
                    });
            });

            if ui.button("Fetch Rate").clicked() {
                let pair = format!("{}{}", self.base_currency, self.target_currency);
                let api_key = self.api_key.clone();
                let tx = self.fetch_rate_tx.clone();
                thread::spawn(move || {
                    let rate = fetch_forex_rate(&api_key, &pair).ok();
                    tx.send(rate).ok();
                });
            }

            if ui.button("Fetch Historical Rates").clicked() {
                let pair = format!("{}{}", self.base_currency, self.target_currency);
                let api_key = self.api_key.clone();
                let tx = self.fetch_rate_tx.clone();
                thread::spawn({
                    let mut trend = self.trend.clone();
                    move || {
                        let rates = fetch_historical_rates(&api_key, &pair).ok();
                        if let Some(rates) = rates {
                            for (_, rate) in rates {
                                trend.push(rate);
                            }
                        }
                        tx.send(Some(0.0)).ok(); // Dummy send to trigger update
                    }
                });
            }

            match self.fetch_rate_rx.try_recv() {
                Ok(rate) => {
                    if let Some(rate) = rate {
                        self.rate = Some(rate);
                    }
                }
                _ => (),
            }

            if let Some(rate) = self.rate {
                ui.label(format!("Rate: {}", rate));
            } else {
                ui.label("Rate: Not fetched");
            }

            if !self.trend.is_empty() {
                let values: PlotPoints = self.trend.iter().enumerate().map(|(i, &y)| [i as f64, y]).collect();
                let line = Line::new(values);
                Plot::new("trend_plot").view_aspect(2.0).show(ui, |plot_ui| {
                    plot_ui.line(line);
                });
            }

            if !self.historical_rates.is_empty() {
                let values: PlotPoints = self
                    .historical_rates
                    .iter()
                    .enumerate()
                    .map(|(i, &(_, rate))| [i as f64, rate])
                    .collect();
                let line = Line::new(values);
                Plot::new("historical_plot").view_aspect(2.0).show(ui, |plot_ui| {
                    plot_ui.line(line);
                });
            }
        });
    }
    
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}
    
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
    
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        // We use a bit of transparency so that if the user switches on the
        // `transparent()` option they get immediate results.
        egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32()
    
        // _visuals.window_fill() would also be a natural choice
    }
    
    fn persist_egui_memory(&self) -> bool {
        true
    }
    
    fn raw_input_hook(&mut self, _ctx: &egui::Context, _raw_input: &mut egui::RawInput) {}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions::default();
    Ok(eframe::run_native(
        "Forex Rate Fetcher",
        options,
        Box::new(|_cc| Box::new(App::new())),
    )?)
}
