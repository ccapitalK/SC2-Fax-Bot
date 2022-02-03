use rust_sc2::prelude::*;

use crate::state::{BotState, GetBotState};
use float_ord::FloatOrd;

#[bot]
#[derive(Default)]
pub struct FaxBot {
    pub state: BotState,
}

impl GetBotState for FaxBot {
    fn get_state(&self) -> &BotState {
        &self.state
    }
    fn get_state_mut(&mut self) -> &mut BotState {
        &mut self.state
    }
}

const HATCH_CAP: usize = 2;

impl FaxBot {
    pub fn energy_cost(&self, ability: AbilityId) -> Option<usize> {
        // FIXME: You'd expect something like `Some(self.game_data.abilities.get(&ability)?.energy_cost)`
        //        to work, but apparently the SC2 API doesn't provide ability energy costs anywhere :(
        match ability {
            AbilityId::EffectInjectLarva => Some(25),
            _ => unimplemented!()
        }
    }
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
    fn count_unit(&self, building_id: UnitTypeId) -> usize {
        self.counter().count(building_id) + self.counter().ordered().count(building_id)
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

impl Player for FaxBot {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Zerg)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        let start_location = self.start_location;
        let enemy_starts = self.game_info.start_locations.iter()
            .map(|p| *p)
            .filter(|p| *p != start_location)
            .collect::<Vec<_>>();
        let mut points = self.expansions.iter()
            .map(|e| e.loc)
            .filter(|p| *p != start_location)
            .collect::<Vec<_>>();
        points.sort_by_cached_key(|p| enemy_starts.iter().map(|s| FloatOrd(p.distance(s))).min());
        let mut state = self.get_state_mut();
        state.bases.push(start_location);
        state.desired_workers = 22;
        state.micro.base_locations_by_expansion_order = points;
        println!("Started bot");
        Ok(())
    }
    fn on_step(&mut self, iteration: usize) -> SC2Result<()> {
        self.perform_building(iteration)?;
        self.perform_training(iteration)?;
        self.perform_micro(iteration)
    }
    fn on_end(&self, _result: GameResult) -> SC2Result<()> {
        println!("Finished bot");
        Ok(())
    }
}
