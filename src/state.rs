use rust_sc2::prelude::*;

#[derive(Debug, Default)]
pub struct BotState {
    pub bases: Vec<Point2>,
    pub expansion_order: Vec<rust_sc2::bot::Expansion>,
    pub peak_roaches: usize,
    pub desired_workers: usize,
    pub desired_gasses: usize,
    pub desired_bases: usize,
    pub is_under_attack: bool,
    pub micro: crate::micro::MicroState,
    pub map_info: crate::map::MapInfo,
}

pub trait GetBotState {
    fn get_state(&self) -> &BotState;
    fn get_state_mut(&mut self) -> &mut BotState;
}
