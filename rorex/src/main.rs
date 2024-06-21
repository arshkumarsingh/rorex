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
    let base_currency = &pair[..3];
    let target_currency = &pair[3..];
    let mut rates = Vec::new();
    let end_date = Utc::now().date_naive();
    let start_date = end_date - chrono::Duration::days(30);

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
            currencies: vec!["USD", "EUR", "JPY", "GBP", "AUD", "CAD", "CHF", "CNY"],
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

            if let Ok(rate) = self.fetch_rate_rx.try_recv() {
                if let Some(rate) = rate {
                    self.rate = Some(rate);
                }
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions::default();
    Ok(eframe::run_native(
        "Forex Rate Fetcher",
        options,
        Box::new(|_cc| Box::new(App::new())),
    )?)
}
