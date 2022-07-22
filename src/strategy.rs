use rust_sc2::prelude::*;

use crate::bot::FaxBot;
use float_ord::FloatOrd;

impl FaxBot {
    pub fn determine_state_for_tick(&mut self, _iteration: usize) {
        let structures = &self.units.my.structures;
        let attacking_units = self.units.enemy.units
            .filter(|u| !u.is_worker() && !u.is_flying() && structures.closest_distance(u.position()).unwrap_or(9999.0) < 18.0);
        let is_under_attack = attacking_units.len() >= 2;
        if is_under_attack != self.state.is_under_attack {
            println!("Under attack? {}", is_under_attack);
        }
        self.state.is_under_attack = is_under_attack;
        {
            let mut expansions = self.expansions.clone();
            let mut unaccounted_mineral_workers = (self.state.desired_workers - 3 * self.state.desired_gasses) as isize;
            expansions.sort_by_key(|u| FloatOrd(u.loc.distance(self.start_location)));
            let mut desired_bases = 0;
            for base in expansions {
                if base.alliance == Alliance::Enemy {
                    continue;
                }
                // println!("{} {}", unaccounted_mineral_workers, base.minerals.len());
                if base.minerals.len() > 0 {
                    // We don't have information about minerals in fog of war, so we can't query
                    // them in self.units.mineral_fields yet. We assume they are unmined.
                    let num_mineral_slots = base.minerals.len() as isize;
                    let num_nearly_empty = self.units.mineral_fields.find_tags(&base.minerals)
                        .filter(|u| u.mineral_contents().unwrap() <= 200)
                        .iter().count() as isize;
                    unaccounted_mineral_workers -= 2 * (num_mineral_slots - num_nearly_empty);
                    desired_bases += 1;
                }
                if unaccounted_mineral_workers <= 0 {
                    break;
                }
            }
            self.state.desired_bases = desired_bases;
        }
        self.state.bases = self.units.my.townhalls.iter().map(|th| th.position()).collect();
        // println!("\n\n\nState: {:?}", self.state);
    }
}