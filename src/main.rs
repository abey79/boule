#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui::{vec2, Sense};
use rand::seq::SliceRandom;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
fn run_native() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native("boule", options, Box::new(|_cc| Box::<BouleApp>::default()))
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
                Box::new(|_cc| Box::new(BouleApp::default())),
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
    egui::Color32::RED,
    egui::Color32::GREEN,
    egui::Color32::BLUE,
    egui::Color32::YELLOW,
    egui::Color32::BROWN,
    egui::Color32::KHAKI,
    egui::Color32::LIGHT_RED,
    egui::Color32::LIGHT_GREEN,
    egui::Color32::LIGHT_BLUE,
    egui::Color32::LIGHT_YELLOW,
    egui::Color32::GOLD,
    egui::Color32::BLACK,
    egui::Color32::DARK_BLUE,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Empty,
    Ball(usize),
}

impl Slot {
    pub fn color(&self) -> egui::Color32 {
        match self {
            Slot::Empty => egui::Color32::BLACK.gamma_multiply(0.02),
            Slot::Ball(color_idx) => BALL_COLORS[*color_idx],
        }
    }
}

struct State {
    column_count: usize,
    column_capacity: usize,
    slots: Vec<Slot>,
}

impl State {
    pub fn new(column_count: usize, column_capacity: usize) -> Self {
        let mut slots = vec![Slot::Empty; column_count * column_capacity];
        let color_count = column_count.saturating_sub(1);
        for col in 0..color_count {
            for row in 0..column_capacity {
                slots[col * column_capacity + row] = Slot::Ball(col % BALL_COLORS.len());
            }
        }

        slots[0..color_count * column_capacity].shuffle(&mut rand::thread_rng());

        Self {
            column_count,
            column_capacity,
            slots,
        }
    }

    pub fn slot(&self, row: usize, column: usize) -> Slot {
        self.slots[column * self.column_capacity + row]
    }

    pub fn slot_mut(&mut self, row: usize, column: usize) -> &mut Slot {
        &mut self.slots[column * self.column_capacity + row]
    }

    pub fn is_winning(&self) -> bool {
        (0..self.column_count).into_iter().all(|col| {
            let first = self.slot(0, col);
            (1..self.column_capacity)
                .into_iter()
                .all(|row| self.slot(row, col) == first)
        })
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
                            response.dnd_set_drag_payload((row, col));
                        }

                        let other: Option<Arc<(usize, usize)>> = response.dnd_release_payload();
                        if let Some(other) = other {
                            if *slot == Slot::Empty && col != other.1 {
                                let other_slot = self.slot(other.0, other.1);
                                self.slot_mut(self.first_empty(col).unwrap(), col)
                                    .clone_from(&other_slot);
                                self.slot_mut(other.0, other.1).clone_from(&Slot::Empty);
                            }
                        }

                        // check if we're being dragged
                        let being_dragged = if let Some(ball) =
                            egui::DragAndDrop::payload::<(usize, usize)>(ui.ctx())
                        {
                            *ball == (row, col)
                        } else {
                            false
                        };

                        painter.circle_filled(
                            response.rect.center(),
                            12.0,
                            if being_dragged {
                                Slot::Empty.color()
                            } else {
                                slot.color()
                            },
                        );

                        if is_top && !being_dragged {
                            painter.circle_stroke(
                                response.rect.center(),
                                9.0,
                                (2.0, egui::Color32::WHITE.gamma_multiply(0.5)),
                            );
                        }
                    }

                    ui.end_row();
                }
            });

        if let Some(ball) = egui::DragAndDrop::payload::<(usize, usize)>(ui.ctx()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                ui.painter()
                    .circle_filled(pos, 12.0, self.slot(ball.0, ball.1).color());
            }
        }
    }
}

struct BouleApp {
    column_count: usize,
    column_capacity: usize,
    state: State,
}

impl Default for BouleApp {
    fn default() -> Self {
        Self {
            column_count: 7,
            column_capacity: 7,
            state: State::new(7, 7),
        }
    }
}

// pub trait BoostedApp: eframe::App {
//     fn boosted_update();
//
//     fn util(&self) -> bool{
//         self.persist_egui_memory()
//     }
// }
//
// impl BoostedApp for BouleApp {
//     fn boosted_update() {
//         todo!()
//     }
// }


impl eframe::App for BouleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("config").num_columns(2).show(ui, |ui| {
                ui.label("Column count:");
                ui.add(
                    egui::DragValue::new(&mut self.column_count).clamp_range(1..=BALL_COLORS.len()),
                );
                ui.end_row();

                ui.label("Column capacity:");
                ui.add(egui::DragValue::new(&mut self.column_capacity).clamp_range(2..=20));
                ui.end_row();

                ui.horizontal(|_| {});
                if ui.button("Reset").clicked() {
                    self.state = State::new(self.column_count, self.column_capacity);
                }
                ui.end_row();
            });

            self.state.ui(ui);

            if self.state.is_winning() {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("You won!")
                        .color(egui::Color32::RED)
                        .size(24.0)
                        .strong(),
                );
            }
        });
    }
}
