use rust_sc2::prelude::*;

use crate::bot::FaxBot;

impl FaxBot {
    pub fn determine_state_for_tick(&mut self, _iteration: usize) {
        let structures = &self.units.my.structures;
        let attacking_units = self.units.enemy.units
            .filter(|u| !u.is_worker() && structures.closest_distance(u.position()).unwrap_or(9999.0) < 18.0);
        let is_under_attack = attacking_units.len() >= 2;
        if is_under_attack != self.state.is_under_attack {
            println!("Under attack? {}", is_under_attack);
        }
        self.state.is_under_attack = is_under_attack;
        self.state.bases = self.units.my.townhalls.iter().map(|th| th.position()).collect();
    }
}