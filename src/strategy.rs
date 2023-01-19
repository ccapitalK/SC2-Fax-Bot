use rust_sc2::prelude::*;

use crate::bot::FaxBot;
use float_ord::FloatOrd;

impl FaxBot {
    pub fn vec_away_from_resources(&self, townhall: Point2) -> Point2 {
        // Not using townhall.center because I don't trust it
        let mut resource_positions: Vec<Point2> = self.units.resources.iter().map(|u| u.position()).filter(|p| p.distance(townhall) < 12.0).collect();
        let sum: Point2 = resource_positions.iter().map(|p| *p - townhall).sum();
        // default to up
        let sum = if sum.len() <= 0.000001 { Point2 { x: 0.0, y: 1.0 } } else { sum.normalize() };
        sum * -1.0
    }
    fn num_attacking_enemies(&self, iteration: usize) -> usize {
        let structures = self.state.get_my_recent_structure_positions(iteration);
        let attacking_units = self.state.get_recent_enemy_spotted_information(iteration);
        let attacking_units = attacking_units.iter().filter(|&&(pos, t)| {
            !t.is_worker()
                // FIXME: Ugly
                && t != UnitTypeId::Overlord
                && t != UnitTypeId::OverlordTransport
                && t != UnitTypeId::Overseer
                && structures.iter().closest_distance(pos).unwrap_or(9999.0) < 18.0
        });
        attacking_units.count()
    }
    fn num_threatening_enemies(&self, iteration: usize) -> usize {
        let spawn = self.start_location;
        let enemy_spawns = self
            .game_info
            .start_locations
            .iter()
            .filter(|&&p| p != spawn);
        let threatening_units = self
            .state
            .get_recent_enemy_spotted_information(iteration.saturating_sub(22 * 20));
        let threatening_units = threatening_units.iter().filter(|&&(pos, _t)| {
            spawn.distance(pos) <= enemy_spawns.clone().closest_distance(pos).unwrap() + 9.0
        });
        threatening_units.count()
    }
    pub fn determine_state_for_tick(&mut self, _iteration: usize) {
        self.state
            .update_my_recent_structure_positions(&self.units.my.structures.clone(), _iteration);
        self.state
            .update_recent_enemy_spotted_information(&self.units.enemy.all.clone(), _iteration);
        let is_under_attack = self.num_attacking_enemies(_iteration) >= 2
            || self.num_threatening_enemies(_iteration) >= 4;
        if is_under_attack != self.state.is_under_attack {
            println!("Under attack? {}", is_under_attack);
        }
        self.state.is_under_attack = is_under_attack;
        {
            let mut expansions = self.expansions.clone();
            let mut unaccounted_mineral_workers =
                (self.state.desired_workers - 3 * self.state.desired_gasses) as isize;
            expansions.sort_by_key(|u| FloatOrd(u.loc.distance(self.start_location)));
            let mut desired_bases = 0;
            for base in expansions {
                if base.alliance == Alliance::Enemy {
                    continue;
                }
                // println!("{} {}", unaccounted_mineral_workers, base.minerals.len());
                if !base.minerals.is_empty() {
                    // We don't have information about minerals in fog of war, so we can't query
                    // them in self.units.mineral_fields yet. We assume they are unmined.
                    let num_mineral_slots = base.minerals.len() as isize;
                    let num_nearly_empty = self
                        .units
                        .mineral_fields
                        .find_tags(&base.minerals)
                        .filter(|u| u.mineral_contents().unwrap() <= 200)
                        .iter()
                        .count() as isize;
                    unaccounted_mineral_workers -= 2 * (num_mineral_slots - num_nearly_empty);
                    desired_bases += 1;
                }
                if unaccounted_mineral_workers <= 0 {
                    break;
                }
            }
            self.state.desired_bases = desired_bases;
        }
        self.state.bases = self
            .units
            .my
            .townhalls
            .iter()
            .map(|th| th.position())
            .collect();
        // println!("\n\n\nState: {:?}", self.state);
    }
}
