use eframe::egui;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 350.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Alpaca Trading - API Setup"),
        ..Default::default()
    };

    eframe::run_native(
        "alpaca_setup",
        options,
        Box::new(|_cc| Ok(Box::new(SetupApp::default()))),
    )
}

#[derive(Default)]
struct SetupApp {
    api_key: String,
    api_secret: String,
    environment: String,
    status: Option<String>,
    saved: bool,
}

impl SetupApp {
    fn get_env_path(&self) -> PathBuf {
        let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push(".env");
        path
    }

    fn load_existing(&mut self) {
        let path = self.get_env_path();
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                for line in contents.lines() {
                    if let Some((key, value)) = line.split_once('=') {
                        match key.trim() {
                            "ALPACA_API_KEY" => self.api_key = value.trim().to_string(),
                            "ALPACA_API_SECRET" => self.api_secret = value.trim().to_string(),
                            "ALPACA_ENV" => self.environment = value.trim().to_string(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn save(&mut self) {
        let path = self.get_env_path();
        let content = format!(
            "# Alpaca API Credentials\n# Get your keys at: https://alpaca.markets/\n\nALPACA_API_KEY={}\nALPACA_API_SECRET={}\n\n# Environment: 'paper' for paper trading (default) or 'live' for real trading\nALPACA_ENV={}\n",
            self.api_key, self.api_secret, self.environment
        );

        match fs::write(&path, content) {
            Ok(_) => {
                self.status = Some("✅ Credentials saved successfully!".to_string());
                self.saved = true;
            }
            Err(e) => {
                self.status = Some(format!("❌ Failed to save: {}", e));
            }
        }
    }
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load existing credentials on first frame
        if self.api_key.is_empty() && self.api_secret.is_empty() && !self.saved {
            self.load_existing();
            if self.environment.is_empty() {
                self.environment = "paper".to_string();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🔑 Alpaca API Configuration");
            ui.add_space(10.0);

            ui.label("Enter your Alpaca Markets API credentials:");
            ui.hyperlink_to("Get API keys at alpaca.markets", "https://alpaca.markets/");
            ui.add_space(15.0);

            // API Key field
            ui.label("API Key ID:");
            ui.add(
                egui::TextEdit::singleline(&mut self.api_key)
                    .hint_text("Your Alpaca API key...")
                    .password(true)
                    .desired_width(400.0),
            );
            ui.add_space(10.0);

            // API Secret field
            ui.label("API Secret Key:");
            ui.add(
                egui::TextEdit::singleline(&mut self.api_secret)
                    .hint_text("Your Alpaca API secret...")
                    .password(true)
                    .desired_width(400.0),
            );
            ui.add_space(10.0);

            // Environment selector
            ui.label("Trading Environment:");
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut self.environment,
                    "paper".to_string(),
                    "Paper Trading (Recommended)",
                );
                ui.radio_value(&mut self.environment, "live".to_string(), "Live Trading ⚠️");
            });
            ui.add_space(15.0);

            // Warning for live trading
            if self.environment == "live" {
                ui.colored_label(
                    egui::Color32::RED,
                    "⚠️ Warning: Live trading uses real money!",
                );
                ui.add_space(5.0);
            }

            // Save button
            let can_save = !self.api_key.is_empty() && !self.api_secret.is_empty();
            ui.add_enabled_ui(can_save, |ui| {
                if ui.button("💾 Save Credentials").clicked() {
                    self.save();
                }
            });

            if !can_save {
                ui.colored_label(
                    egui::Color32::GRAY,
                    "Fill in both API Key and Secret to save",
                );
            }

            // Status message
            if let Some(ref status) = self.status {
                ui.add_space(10.0);
                if status.starts_with("✅") {
                    ui.colored_label(egui::Color32::GREEN, status);
                    ui.label("You can now close this window and run: cargo run");
                } else {
                    ui.colored_label(egui::Color32::RED, status);
                }
            }

            ui.add_space(20.0);
            ui.separator();
            ui.small("Credentials are stored locally in .env file");
            ui.small("Never share your API keys or commit .env to version control!");
        });
    }
}
