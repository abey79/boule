#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::{egui, Storage};
use egui::{vec2, Sense};
use rand::seq::SliceRandom;
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
fn run_native() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "boule",
        options,
        Box::new(|cc| {
            Box::new(
                cc.storage
                    .and_then(|storage| eframe::get_value::<BouleApp>(storage, "__app__"))
                    .unwrap_or_default(),
            )
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn run_web() -> Result<(), eframe::Error> {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| {
                    Box::new(
                        cc.storage
                            .and_then(|storage| eframe::get_value::<BouleApp>(storage, "__app__"))
                            .unwrap_or_default(),
                    )
                }),
            )
            .await
            .expect("failed to start eframe");
    });

    Ok(())
}

fn main() -> Result<(), eframe::Error> {
    #[cfg(not(target_arch = "wasm32"))]
    return run_native();

    #[cfg(target_arch = "wasm32")]
    return run_web();
}

const BALL_COLORS: &[egui::Color32] = &[
    // from https://colorkit.co/palette/ffef3e-ffa0c5-a98467-32dba9-ffa600-c7522a-476f95-ffd380-893f71-92ba92/
    egui::Color32::from_rgb(255, 239, 62),  // #ffef3e
    egui::Color32::from_rgb(255, 160, 197), // #ffa0c5
    egui::Color32::from_rgb(169, 132, 103), // #a98467
    egui::Color32::from_rgb(50, 219, 169),  // #32dba9
    egui::Color32::from_rgb(255, 166, 0),   // #ffa600
    egui::Color32::from_rgb(199, 82, 42),   // #c7522a
    egui::Color32::from_rgb(71, 111, 149),  // #476f95
    egui::Color32::from_rgb(255, 211, 128), // #ffd380
    egui::Color32::from_rgb(137, 63, 113),  // #893f71
    egui::Color32::from_rgb(146, 186, 146), // #92ba92
];

enum BallTheme {
    Plain,
    Hole,
}

impl BallTheme {
    pub fn from_index(index: usize) -> Self {
        match index % 2 {
            0 => BallTheme::Plain,
            1 => BallTheme::Hole,
            _ => unreachable!(),
        }
    }
}

struct BallStyle {
    color: egui::Color32,
    theme: BallTheme,
}

impl BallStyle {
    const MAX_STYLES: usize = BALL_COLORS.len() * 2;

    pub fn paint(&self, painter: &egui::Painter, pos: egui::Pos2) {
        match self.theme {
            BallTheme::Plain => {
                painter.circle_filled(pos, 12.0, self.color);
            }
            BallTheme::Hole => {
                painter.circle_stroke(pos, 8.0, (8.0, self.color));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
enum Slot {
    Empty,
    Ball(usize),
}

impl Slot {
    pub fn color(&self) -> BallStyle {
        match self {
            Slot::Empty => BallStyle {
                color: egui::Color32::BLACK.gamma_multiply(0.02),
                theme: BallTheme::Plain,
            },
            Slot::Ball(color_idx) => BallStyle {
                color: BALL_COLORS[*color_idx % BALL_COLORS.len()],
                theme: BallTheme::from_index(*color_idx / BALL_COLORS.len()),
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct State {
    column_count: usize,
    column_capacity: usize,
    play_count: usize,
    slots: Vec<Slot>,
}

impl State {
    pub fn new(column_count: usize, column_capacity: usize) -> Self {
        let mut slots = vec![Slot::Empty; column_count * column_capacity];
        let color_count = column_count.saturating_sub(1);
        for col in 0..color_count {
            for row in 0..column_capacity {
                slots[col * column_capacity + row] = Slot::Ball(col);
            }
        }

        slots[0..color_count * column_capacity].shuffle(&mut rand::thread_rng());

        Self {
            column_count,
            column_capacity,
            play_count: 0,
            slots,
        }
    }

    pub fn slot(&self, row: usize, column: usize) -> Slot {
        self.slots[column * self.column_capacity + row]
    }

    fn slot_mut(&mut self, row: usize, column: usize) -> &mut Slot {
        &mut self.slots[column * self.column_capacity + row]
    }

    pub fn move_ball(&mut self, from_column: usize, to_column: usize) {
        if from_column == to_column {
            return;
        }

        if let (Some(from_row), Some(to_row)) =
            (self.first_ball(from_column), self.first_empty(to_column))
        {
            let ball = self.slot(from_row, from_column);
            self.slot_mut(to_row, to_column).clone_from(&ball);
            self.slot_mut(from_row, from_column)
                .clone_from(&Slot::Empty);
            self.play_count += 1;
        }
    }

    // return play count if winning
    pub fn is_winning(&self) -> Option<usize> {
        if (0..self.column_count).into_iter().all(|col| {
            let first = self.slot(0, col);
            (1..self.column_capacity)
                .into_iter()
                .all(|row| self.slot(row, col) == first)
        }) {
            Some(self.play_count)
        } else {
            None
        }
    }

    pub fn is_top(&self, row: usize, column: usize) -> bool {
        (0..row)
            .into_iter()
            .all(|row| self.slot(row, column) == Slot::Empty)
            && self.slot(row, column) != Slot::Empty
    }

    pub fn first_empty(&self, column: usize) -> Option<usize> {
        (0..self.column_capacity)
            .rev()
            .into_iter()
            .find(|&row| self.slot(row, column) == Slot::Empty)
    }

    pub fn first_ball(&self, column: usize) -> Option<usize> {
        (0..self.column_capacity)
            .into_iter()
            .find(|&row| self.slot(row, column) != Slot::Empty)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("board")
            .num_columns(self.column_count)
            .spacing(egui::Vec2::ZERO)
            .show(ui, |ui| {
                for row in 0..self.column_capacity {
                    for col in 0..self.column_count {
                        let slot = &self.slot(row, col);
                        let is_top = self.is_top(row, col);

                        let (response, painter) =
                            ui.allocate_painter(vec2(30.0, 30.0), Sense::drag());

                        if is_top {
                            response.dnd_set_drag_payload(col);
                        }

                        let other: Option<Arc<usize>> = response.dnd_release_payload();
                        if let Some(other_col) = other {
                            self.move_ball(*other_col, col);
                        }

                        // check if we're being dragged
                        let being_dragged = if let Some(dragged_col) =
                            egui::DragAndDrop::payload::<usize>(ui.ctx())
                        {
                            if let Some(dragged_row) = self.first_ball(*dragged_col) {
                                (row, col) == (dragged_row, *dragged_col)
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        if being_dragged {
                            Slot::Empty.color().paint(&painter, response.rect.center());
                        } else {
                            slot.color().paint(&painter, response.rect.center());
                        }
                    }

                    ui.end_row();
                }
            });

        if let Some(dragged_col) = egui::DragAndDrop::payload::<usize>(ui.ctx()) {
            if let Some(dragged_row) = self.first_ball(*dragged_col) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    self.slot(dragged_row, *dragged_col)
                        .color()
                        .paint(ui.painter(), pos);
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct BouleApp {
    column_count: usize,
    column_capacity: usize,
    state: Option<State>,

    #[serde(skip)]
    auto_save: bool,
}

impl Default for BouleApp {
    fn default() -> Self {
        Self {
            column_count: 7,
            column_capacity: 7,
            state: None,
            auto_save: false,
        }
    }
}

impl eframe::App for BouleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let old_self = self.clone();

            let mut reset = false;
            if let Some(state) = &mut self.state {
                state.ui(ui);

                reset = if let Some(play_count) = state.is_winning() {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(format!("You won in {} moves!", play_count))
                            .color(egui::Color32::RED)
                            .size(24.0)
                            .strong(),
                    );
                    ui.button("Restart").clicked()
                } else {
                    ui.button("Abort").clicked()
                };
            } else {
                egui::Grid::new("config").num_columns(2).show(ui, |ui| {
                    ui.label("Column count:");
                    ui.add(
                        egui::DragValue::new(&mut self.column_count)
                            .clamp_range(1..=(BallStyle::MAX_STYLES + 1)),
                    );
                    ui.end_row();

                    ui.label("Column capacity:");
                    ui.add(egui::DragValue::new(&mut self.column_capacity).clamp_range(2..=20));
                    ui.end_row();

                    ui.horizontal(|_| {});
                    if ui
                        .button(egui::RichText::new("Start").strong().size(18.0))
                        .clicked()
                    {
                        self.state = Some(State::new(self.column_count, self.column_capacity));
                    }
                    ui.end_row();
                });
            }

            if reset {
                self.state = None;
            }

            // trigger saving if state changed
            if *self != old_self {
                self.auto_save = true;
            }
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, "__app__", self);
        self.auto_save = false;
    }

    fn auto_save_interval(&self) -> Duration {
        if self.auto_save {
            Duration::from_secs(0)
        } else {
            Duration::from_secs(30)
        }
    }
}
