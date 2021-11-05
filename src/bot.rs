use rust_sc2::prelude::*;

use crate::util;

#[bot]
#[derive(Default)]
pub struct FaxBot {
    peak_lings: usize,
}

const WORKER_CAP: usize = 16;
const HATCH_CAP: usize = 2;

impl FaxBot {
    fn energy_cost(&self, ability: AbilityId) -> Option<usize> {
        Some(25) //Some(self.game_data.abilities.get(&ability)?.energy_cost)
    }
    fn calculate_pending_supply(&self) -> usize {
        let supply_unit = self.race_values.supply;
        let supply_per_provider = self.game_data.units.get(&supply_unit).unwrap().food_provided;
        (self.supply_cap as usize) + (supply_per_provider.floor() as usize) * self.counter().ordered().count(supply_unit)
    }
    fn count_unit(&self, building_id: UnitTypeId) -> usize {
        self.counter().count(building_id) + self.counter().ordered().count(building_id)
    }
    fn perform_building(&mut self, _iteration: usize) -> SC2Result<()> {
        let main_base = self.start_location.towards(self.game_info.map_center, 5.0);
        if self.count_unit(UnitTypeId::SpawningPool) < 1 {
            self.create_building(UnitTypeId::SpawningPool, main_base);
        } else {
            let num_hatcheries = self.count_unit(UnitTypeId::Hatchery);
            if num_hatcheries < HATCH_CAP {
                self.create_building(UnitTypeId::Hatchery, main_base);
            }
        }
        Ok(())
    }
    fn create_building(&mut self, unit_type: UnitTypeId, main_base: Point2) {
        if let Some(w) = self.units.my.workers.first() {
            if let Some(location) = self.find_placement(unit_type, main_base, Default::default()) {
                w.build(unit_type, location, false);
            }
        }
    }
    fn perform_training(&mut self, _iteration: usize) -> SC2Result<()> {
        for l in self.units.my.larvas.idle() {
            let num_workers = self.count_unit(UnitTypeId::Drone);
            if self.calculate_pending_supply() < std::cmp::min(self.supply_used as usize + 4, 200) {
                if self.can_afford(UnitTypeId::Overlord, false) {
                    l.train(UnitTypeId::Overlord, false);
                }
            } else if num_workers < WORKER_CAP && self.can_afford(UnitTypeId::Drone, true) {
                l.train(UnitTypeId::Drone, false);
            } else if self.can_afford(UnitTypeId::Zergling, true) {
                l.train(UnitTypeId::Zergling, false);
            }
        }
        if self.count_unit(UnitTypeId::SpawningPool) > 0 && self.count_unit(UnitTypeId::Queen) < HATCH_CAP {
            // TODO: This should be chosen by lowest amount of queued units
            if let Some(hatch) = self.units.my.townhalls.min(|hatch| 1) {
                hatch.train(UnitTypeId::Queen, true);
            }
        }
        Ok(())
    }
    fn perform_micro(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_lings = self.counter().count(UnitTypeId::Zergling);
        if num_lings > self.peak_lings {
            util::a_move(&self.units.my.units.of_type(UnitTypeId::Zergling).idle(), self.enemy_start, false);
            self.peak_lings = num_lings;
        }
        for queen in &self.units.my.units.filter(|q| q.type_id() == UnitTypeId::Queen
            && q.energy().unwrap() as usize >= self.energy_cost(AbilityId::EffectInjectLarva).unwrap()
            && q.is_idle()
        ) {
            let idle_hatcheries = self.units.my.townhalls.filter(
                |hatch| !hatch.buffs().contains(&BuffId::QueenSpawnLarvaTimer));
            if let Some(hatch) = idle_hatcheries.closest(queen.position()) {
                println!("Going to inject");
                queen.command(AbilityId::EffectInjectLarva, Target::Tag(hatch.tag()), true);
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
