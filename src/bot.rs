use rust_sc2::prelude::*;

use crate::util;
use crate::state::{BotState, GetBotState};

#[bot]
#[derive(Default)]
pub struct FaxBot {
    state: BotState,
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
    fn energy_cost(&self, ability: AbilityId) -> Option<usize> {
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
    fn perform_building(&mut self, _iteration: usize) -> SC2Result<()> {
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
    fn perform_training(&mut self, _iteration: usize) -> SC2Result<()> {
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
    fn allocate_workers(&mut self) -> SC2Result<()> {
        let mut surplus_workers = Units::new();
        let mut undermined_resources = vec![];
        surplus_workers.extend(self.units.my.workers.idle());
        for base in self.units.my.townhalls.ready() {
            let assigned = base.assigned_harvesters().unwrap();
            let ideal = base.ideal_harvesters().unwrap();
            if assigned < ideal {
                let mineral_patch = self.units.mineral_fields.closest(base.position()).unwrap();
                for _ in 0..(ideal - assigned) {
                    undermined_resources.push(mineral_patch.tag());
                }
            } else if assigned > ideal {
                let local_minerals = self.units.mineral_fields.iter().closer(11.0, base.position()).map(|u| u.tag()).collect::<Vec<_>>();
                let nearby_miners = self.units.my.workers
                    .filter(|u| u.target_tag().map_or(
                        false,
                        |tag| local_minerals.contains(&tag) || (u.is_carrying_minerals() && tag == base.tag())));
                surplus_workers.extend(nearby_miners.iter().take((assigned - ideal) as usize).cloned());
            }
        }
        for gas in &self.units.my.gas_buildings {
            let assigned = gas.assigned_harvesters().unwrap();
            let ideal = gas.ideal_harvesters().unwrap();
            if assigned < ideal {
                for _ in 0..(ideal - assigned) {
                    undermined_resources.push(gas.tag());
                }
            }
        }
        for (worker, resource) in surplus_workers.iter().zip(undermined_resources.iter()) {
            worker.gather(*resource, false);
        }
        Ok(())
    }
    fn perform_micro(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_roaches = self.counter().count(UnitTypeId::Roach);
        if num_roaches > self.state.peak_roaches {
            println!("A moving with {} roaches", num_roaches);
            util::a_move(&self.units.my.units.of_type(UnitTypeId::Roach).idle(), self.enemy_start, false);
            self.state.peak_roaches = num_roaches;
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
        self.allocate_workers()?;
        Ok(())
    }
}

impl Player for FaxBot {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Zerg)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        let start_location = self.start_location;
        let mut state = self.get_state_mut();
        state.bases.push(start_location);
        state.desired_workers = 22;
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
