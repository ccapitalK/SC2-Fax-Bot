use rust_sc2::prelude::*;

use crate::state::{BotState, GetBotState};
use crate::bot::FaxBot;

const HATCH_CAP: usize = 2;

impl FaxBot {
    fn ensure_taken_gasses(&mut self, num_gasses: usize) {
        let current_gasses = self.count_unit(UnitTypeId::Extractor);
        let mut build_locations = vec![];
        for base in self.state.bases.iter() {
            if current_gasses < num_gasses {
                if let Some(nearest_free_gas) = self.find_gas_placement(*base) {
                    build_locations.push(nearest_free_gas.position());
                    if let Some(w) = self.units.my.workers.first() {
                        w.build_gas(nearest_free_gas.tag(), false);
                    }
                }
            }
        }
    }

    fn least_busy_hatch(&self) -> Option<Unit> {
        self.units.my.townhalls.filter(|hatch| hatch.is_ready() && hatch.orders().len() < 5).min(|hatch| hatch.orders().len()).map(|h| h.to_owned())
    }

    fn calculate_pending_supply(&self) -> usize {
        let supply_unit = self.race_values.supply;
        let supply_per_provider = self.game_data.units.get(&supply_unit).unwrap().food_provided;
        (self.supply_cap as usize) + (supply_per_provider.floor() as usize) * self.counter().ordered().count(supply_unit)
    }

    pub fn perform_building(&mut self, _iteration: usize) -> SC2Result<()> {
        let main_base = self.start_location.towards(self.game_info.map_center, 5.0);
        let num_hatcheries = self.count_unit(UnitTypeId::Hatchery);
        if self.count_unit(UnitTypeId::SpawningPool) < 1 {
            self.create_building(UnitTypeId::SpawningPool, main_base);
        } else if num_hatcheries < HATCH_CAP {
            self.create_building(UnitTypeId::Hatchery, main_base);
        } else if self.count_unit(UnitTypeId::Extractor) < 2 {
            self.ensure_taken_gasses(2);
        } else if self.count_unit(UnitTypeId::RoachWarren) < 1 {
            self.create_building(UnitTypeId::RoachWarren, main_base);
        }
        Ok(())
    }

    fn create_building(&mut self, unit_type: UnitTypeId, location: Point2) {
        if let Some(w) = self.units.my.workers.first() {
            if let Some(location) = self.find_placement(unit_type, location, Default::default()) {
                w.build(unit_type, location, false);
            }
        }
    }

    pub fn perform_training(&mut self, _iteration: usize) -> SC2Result<()> {
        for l in self.units.my.larvas.idle() {
            let num_workers = self.supply_workers as usize + self.counter().ordered().count(UnitTypeId::Drone);
            if self.calculate_pending_supply() < std::cmp::min(self.supply_used as usize + 4, 200) {
                if self.can_afford(UnitTypeId::Overlord, false) {
                    l.train(UnitTypeId::Overlord, false);
                }
            } else if num_workers < self.state.desired_workers && self.can_afford(UnitTypeId::Drone, true) {
                l.train(UnitTypeId::Drone, false);
            } else if self.can_afford(UnitTypeId::Roach, true) {
                l.train(UnitTypeId::Roach, false);
            }
        }
        if self.count_unit(UnitTypeId::SpawningPool) > 0 && self.count_unit(UnitTypeId::Queen) < HATCH_CAP {
            if let Some(hatch) = self.least_busy_hatch() {
                hatch.train(UnitTypeId::Queen, true);
            }
        }
        Ok(())
    }
}