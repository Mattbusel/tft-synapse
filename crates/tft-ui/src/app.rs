//! Main egui app: wires UI together, manages the background polling loop.

use crate::overlay;
use crate::panels::{
    augment_panel, carry_panel, economy_panel, item_panel, lobby_panel, pool_panel,
    positioning_panel, review_panel, round_panel, stats_panel, status_bar,
};
use crate::state::{ConnectionStatus, UiState};
use crate::theme;
use egui::Context;
use std::path::PathBuf;
use std::sync::mpsc;
use tft_advisor::Advisor;
use tft_types::GameState;
use tracing::warn;

pub enum AppMessage {
    GameStateUpdate(GameState),
    Error(String),
    Disconnected,
    /// A newer version is available for download.
    UpdateAvailable {
        version: String,
        url: String,
    },
}

pub struct TftSynapseApp {
    ui_state: UiState,
    advisor: Advisor,
    rx: mpsc::Receiver<AppMessage>,
}

impl TftSynapseApp {
    pub fn new(
        model_path: PathBuf,
        rx: mpsc::Receiver<AppMessage>,
    ) -> Result<Self, tft_types::TftError> {
        let advisor = Advisor::new(model_path)?;
        Ok(Self {
            ui_state: UiState::new(),
            advisor,
            rx,
        })
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::GameStateUpdate(state) => {
                    self.ui_state.set_connected(ConnectionStatus::Connected);

                    match self.advisor.advise_full(&state) {
                        Ok(full) => {
                            self.ui_state.recommendation = full.augment.clone();
                            self.ui_state.full_recommendation = Some(full);
                        }
                        Err(e) => {
                            warn!("Advisor error: {}", e);
                            self.ui_state.last_error = Some(e.to_string());
                            self.ui_state.recommendation = None;
                            self.ui_state.full_recommendation = None;
                        }
                    }

                    self.ui_state.games_trained = self.advisor.games_trained();
                    self.ui_state.game_state = Some(state);
                }
                AppMessage::Error(e) => {
                    self.ui_state.last_error = Some(e);
                    self.ui_state.set_connected(ConnectionStatus::Polling);
                }
                AppMessage::Disconnected => {
                    self.ui_state.set_connected(ConnectionStatus::Disconnected);
                }
                AppMessage::UpdateAvailable { version, url } => {
                    self.ui_state.update_available = Some((version, url));
                }
            }
        }
    }

    /// Build the default export path `~/.tft-synapse/<filename>`.
    fn export_path(filename: &str) -> PathBuf {
        let base = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".tft-synapse");
        base.join(filename)
    }

    fn handle_export(&mut self) {
        let history_path = Self::export_path("history.csv");
        let stats_path = Self::export_path("stats.csv");

        match tft_advisor::export_history_csv(&self.advisor.metrics, &history_path) {
            Ok(n) => {
                match tft_advisor::export_stats_csv(
                    &self.advisor.metrics,
                    self.ui_state.games_trained,
                    &stats_path,
                ) {
                    Ok(()) => {
                        self.ui_state.last_info = Some(format!(
                            "Exported {} games to {}",
                            n,
                            history_path.display()
                        ));
                        self.ui_state.last_error = None;
                    }
                    Err(e) => {
                        self.ui_state.last_error = Some(format!("Stats export error: {}", e));
                    }
                }
            }
            Err(e) => {
                self.ui_state.last_error = Some(format!("History export error: {}", e));
            }
        }
    }
}

impl eframe::App for TftSynapseApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.drain_messages();

        // F9 toggles click-through overlay mode.
        if ctx.input(|i| i.key_pressed(egui::Key::F9)) {
            self.ui_state.toggle_click_through();
        }

        // Apply overlay settings if they changed.
        if self.ui_state.overlay_dirty {
            if let Err(e) = overlay::apply_overlay(&self.ui_state.overlay_config) {
                warn!("overlay apply error: {}", e);
            }
            self.ui_state.overlay_dirty = false;
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        let style = ctx.style();
        let mut visuals = style.visuals.clone();
        visuals.window_fill = theme::BG_DARK;
        visuals.panel_fill = theme::BG_DARK;
        ctx.set_visuals(visuals);

        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            status_bar::render(
                ui,
                self.ui_state.game_state.as_ref(),
                self.ui_state.connection_status.as_ref(),
            );
        });

        egui::SidePanel::right("stats_panel")
            .min_width(180.0)
            .show(ctx, |ui| {
                let export_clicked = stats_panel::render(
                    ui,
                    &self.advisor.metrics,
                    self.ui_state.games_trained,
                    self.ui_state.connection_status.as_ref(),
                );
                if export_clicked {
                    self.handle_export();
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some((ref ver, ref url)) = self.ui_state.update_available {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Update available: v{}", ver))
                            .color(theme::SCORE_HIGH)
                            .strong()
                            .small(),
                    );
                    ui.hyperlink_to("Download", url);
                });
                ui.separator();
            }

            augment_panel::render(ui, self.ui_state.recommendation.as_ref());

            if let Some(ref full) = self.ui_state.full_recommendation {
                ui.add_space(4.0);
                egui::CollapsingHeader::new("Economy")
                    .default_open(true)
                    .show(ui, |ui| {
                        economy_panel::render(ui, &full.economy);
                    });
                egui::CollapsingHeader::new("Carry Targets")
                    .default_open(true)
                    .show(ui, |ui| {
                        carry_panel::render(ui, &full.carries);
                    });
                egui::CollapsingHeader::new("Stage Awareness")
                    .default_open(true)
                    .show(ui, |ui| {
                        round_panel::render(ui, &full.stage_awareness);
                    });
                egui::CollapsingHeader::new("Pool Tracker")
                    .default_open(false)
                    .show(ui, |ui| {
                        pool_panel::render(ui, &full.pool);
                    });
                egui::CollapsingHeader::new("Positioning")
                    .default_open(false)
                    .show(ui, |ui| {
                        positioning_panel::render(ui, &full.positions);
                    });
                egui::CollapsingHeader::new("Items")
                    .default_open(false)
                    .show(ui, |ui| {
                        item_panel::render(ui, &full.items);
                    });
                egui::CollapsingHeader::new("Lobby")
                    .default_open(false)
                    .show(ui, |ui| {
                        lobby_panel::render(ui, &full.lobby);
                    });
                egui::CollapsingHeader::new("Game Review")
                    .default_open(false)
                    .show(ui, |ui| {
                        review_panel::render(ui, &full.review);
                    });
            }

            if let Some(ref info) = self.ui_state.last_info {
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(info.as_str())
                        .color(theme::SCORE_HIGH)
                        .small(),
                );
            }

            if let Some(ref err) = self.ui_state.last_error {
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Error: {}", err))
                        .color(theme::SCORE_LOW)
                        .small(),
                );
            }

            // Overlay settings collapsible panel.
            ui.add_space(8.0);
            egui::CollapsingHeader::new("Overlay Settings")
                .default_open(false)
                .show(ui, |ui| {
                    let mut opacity = self.ui_state.overlay_config.opacity;
                    if ui
                        .add(egui::Slider::new(&mut opacity, 0.1..=1.0).text("Opacity"))
                        .changed()
                    {
                        self.ui_state.set_opacity(opacity);
                    }

                    let mut click_through = self.ui_state.overlay_config.click_through;
                    if ui
                        .checkbox(&mut click_through, "Click-through (F9)")
                        .changed()
                    {
                        self.ui_state.toggle_click_through();
                    }

                    ui.label(
                        egui::RichText::new(format!(
                            "Overlay: opacity={:.0}%  click-through={}",
                            self.ui_state.overlay_config.opacity * 100.0,
                            self.ui_state.overlay_config.click_through,
                        ))
                        .small()
                        .color(theme::TEXT_SECONDARY),
                    );
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_message_error_variant() {
        let msg = AppMessage::Error("test".to_string());
        let is_error = matches!(msg, AppMessage::Error(_));
        assert!(is_error);
    }

    #[test]
    fn test_app_message_disconnected_variant() {
        let _msg = AppMessage::Disconnected;
    }

    #[test]
    fn test_ui_state_default_no_recommendation() {
        let state = UiState::new();
        assert!(!state.has_recommendation());
        assert!(state.game_state.is_none());
    }

    #[test]
    fn test_export_path_contains_filename() {
        let p = TftSynapseApp::export_path("history.csv");
        assert!(p.to_string_lossy().contains("history.csv"));
        assert!(p.to_string_lossy().contains(".tft-synapse"));
    }
}
