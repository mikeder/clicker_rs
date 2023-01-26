use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Duration;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct App {
    // title of the app window
    title: String,

    leader_refresh_rate: Duration,

    state: Arc<Mutex<State>>,
}

struct State {
    last_updated: chrono::DateTime<chrono::Utc>,
    learder_board: Vec<Leader>,
    status_online: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_updated: chrono::Utc::now(),
            learder_board: vec![],
            status_online: false,
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let leader_refresh_rate = chrono::Duration::seconds(10);
        let state = Arc::new(Mutex::new(State::default()));

        thread::spawn({
            let state = state.clone();
            move || loop {
                let now = chrono::Utc::now();

                let last_updated = {
                    let state = state.lock().unwrap();
                    state.last_updated
                };

                if now.signed_duration_since(last_updated) > leader_refresh_rate {
                    let state = state.clone();

                    let req = ehttp::Request {
                        method: "POST".to_string(),
                        url: "https://click.sqweeb.net/shitlist.v1.ShitlistService/Leaders"
                            .to_string(),
                        body: "{}".as_bytes().to_vec(),
                        headers: ehttp::headers(&[("Content-Type", "application/json")]),
                    };

                    ehttp::fetch(req, move |result: ehttp::Result<ehttp::Response>| {
                        match result {
                            Err(e) => {
                                println!("fetch leaders: {}", e);
                                let mut state = state.lock().unwrap();
                                state.status_online = false;
                                state.last_updated = now;
                                return;
                            }
                            Ok(_) => {}
                        }
                        let res = result.unwrap();
                        println!("Status code: {:?}", res.status);
                        println!("Response: {:?}", std::str::from_utf8(&res.bytes));

                        let res: Result<LeaderResponse, serde_json::Error> =
                            serde_json::from_slice(&res.bytes);
                        match res {
                            Ok(new_leaders) => {
                                let mut state = state.lock().unwrap();
                                state.learder_board = new_leaders.top_clickers;
                                state.last_updated = now;
                                state.status_online = true;
                            }
                            Err(e) => {
                                let mut state = state.lock().unwrap();
                                println!("{}", e);
                                state.last_updated = now;
                                state.status_online = false;
                            }
                        };
                    });
                }
            }
        });

        Self {
            title: ("Rusty Clicker").to_string(),
            leader_refresh_rate: leader_refresh_rate,
            state: state,
        }
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            title,
            leader_refresh_rate,
            state,
        } = self;

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading(title);

            ui.label("Add login here.");
            ui.label("Add button to click here.");

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                let state = state.lock().unwrap();
                if state.status_online {
                    ui.label("online");
                } else {
                    ui.label("offline");
                }
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Top Clickers:");

            let state = state.lock().unwrap();
            egui::Grid::new("top_clickers").show(ui, |ui| {
                ui.label("User ID");
                ui.label("Clicks");
                ui.end_row();

                let leaders = &state.learder_board;
                for clicker in leaders {
                    ui.label(&clicker.user_id);
                    ui.label(&clicker.clicks);
                    ui.end_row();
                }
            });
            egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }
}

/// a struct into which to decode the thing
#[derive(serde::Deserialize, serde::Serialize)]
struct LeaderResponse {
    #[serde(rename(serialize = "topClickers", deserialize = "topClickers"))]
    top_clickers: Vec<Leader>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Leader {
    #[serde(rename(serialize = "userId", deserialize = "userId"))]
    user_id: String,
    clicks: String,
}
