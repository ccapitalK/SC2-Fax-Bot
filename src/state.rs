use rust_sc2::prelude::*;

#[derive(Default)]
pub struct BotState {
    pub bases: Vec<Point2>,
    pub expansion_order: Vec<rust_sc2::bot::Expansion>,
    pub peak_roaches: usize,
    pub desired_workers: usize,
    pub micro: crate::micro::MicroState,
}

pub trait GetBotState {
    fn get_state(&self) -> &BotState;
    fn get_state_mut(&mut self) -> &mut BotState;
}
