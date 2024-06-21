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

fn fetch_forex_rate(api_key: &str, pair: &str) -> Result<f64, String> {
    let url = format!("https://v6.exchangerate-api.com/v6/{}/latest/{}", api_key, &pair[..3]);

    let client = Client::new();
    let response = client.get(&url).send().map_err(|e| e.to_string())?;
    let forex_response: ForexResponse = response.json().map_err(|e| e.to_string())?;

    let target_currency = &pair[3..];
    forex_response.conversion_rates.get(target_currency).copied().ok_or_else(|| {
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

struct App {
    api_key: String,
    base_currency: String,
    target_currency: String,
    rate: Option<f64>,
    fetch_rate_tx: Sender<Option<f64>>,
    fetch_rate_rx: Receiver<Option<f64>>,
    currencies: Vec<&'static str>,
    trend: Vec<f64>,
    historical_rates: Vec<(NaiveDate, f64)>,
}

impl App {
    fn new() -> Self {
        let (fetch_rate_tx, fetch_rate_rx) = mpsc::channel();
        App {
            api_key: "".to_string(),
            base_currency: "USD".to_string(),
            target_currency: "EUR".to_string(),
            rate: None,
            fetch_rate_tx,
            fetch_rate_rx,
            currencies: vec![
                "USD", "EUR", "JPY", "GBP", "AUD", "CAD", "CHF", "CNY", "AED", "AFN", "ALL", "AMD", "ANG",
                "AOA", "ARS", "AWG", "AZN", "BAM", "BBD", "BDT", "BGN", "BHD", "BIF", "BMD", "BND", "BOB",
                "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CDF", "CLP", "COP", "CRC", "CUP", "CVE", "CZK",
                "DJF", "DKK", "DOP", "DZD", "EGP", "ERN", "ETB", "FJD", "FKP", "FOK", "GEL", "GGP", "GHS",
                "GIP", "GMD", "GNF", "GTQ", "GYD", "HKD", "HNL", "HRK", "HTG", "HUF", "IDR", "ILS", "IMP",
                "INR", "IQD", "IRR", "ISK", "JEP", "JMD", "JOD", "KES", "KGS", "KHR", "KID", "KMF", "KRW",
                "KWD", "KYD", "KZT", "LAK", "LBP", "LKR", "LRD", "LSL", "LYD", "MAD", "MDL", "MGA", "MKD",
                "MMK", "MNT", "MOP", "MRU", "MUR", "MVR", "MWK", "MXN", "MYR", "MZN", "NAD", "NGN", "NIO",
                "NOK", "NPR", "NZD", "OMR", "PAB", "PEN", "PGK", "PHP", "PKR", "PLN", "PYG", "QAR", "RON",
                "RSD", "RUB", "RWF", "SAR", "SBD", "SCR", "SDG", "SEK", "SGD", "SHP", "SLE", "SLL", "SOS",
                "SRD", "SSP", "STN", "SYP", "SZL", "THB", "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TVD",
                "TWD", "TZS", "UAH", "UGX", "UYU", "UZS", "VES", "VND", "VUV", "WST", "XAF", "XCD", "XDR",
                "XOF", "XPF", "YER", "ZAR", "ZMW", "ZWL",
            ],
            trend: Vec::new(),
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
