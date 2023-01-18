use rust_sc2::prelude::*;

use crate::bot::FaxBot;

impl FaxBot {
    /// Returns whether or not gasses were taken this iteration
    fn ensure_taken_gasses(&mut self, num_gasses: usize) -> bool {
        if self.count_unit(UnitTypeId::Extractor) >= self.state.desired_gasses {
            return false;
        }
        let mut available_workers = self.units.my.workers.iter().collect::<Vec<_>>();
        available_workers.sort_by_key(|u| u.is_idle());
        let current_gasses = self
            .units
            .my
            .structures
            .filter(|u| {
                u.type_id() == UnitTypeId::Extractor
                    && u.vespene_contents().map(|v| v > 0).unwrap_or(false)
            })
            .len()
            + self.counter().ordered().count(UnitTypeId::Extractor);
        if self.can_afford(UnitTypeId::Extractor, false) && current_gasses < num_gasses {
            if let Some(w) = available_workers.pop() {
                for base in self.state.bases.iter() {
                    if let Some(nearest_free_gas) = self.find_gas_placement(*base) {
                        w.build_gas(nearest_free_gas.tag(), false);
                        self.state.micro.drones.insert(w.tag(), crate::micro::DroneTask::Construct { queued_at: self.current_iteration });
                        self.subtract_resources(UnitTypeId::Extractor, false);
                        return true;
                    }
                }
            }
        }
        false
    }

    fn least_busy_hatch(&self) -> Option<Unit> {
        self.units
            .my
            .townhalls
            .filter(|hatch| hatch.is_ready() && hatch.orders().len() < 5)
            .min(|hatch| hatch.orders().len())
            .map(|h| h.to_owned())
    }

    fn calculate_pending_supply(&self) -> usize {
        let supply_unit = self.race_values.supply;
        let supply_per_provider = self
            .game_data
            .units
            .get(&supply_unit)
            .unwrap()
            .food_provided;
        (self.supply_cap as usize)
            + (supply_per_provider.floor() as usize) * self.counter().ordered().count(supply_unit)
    }

    fn determine_best_expansion_order(&self) -> Vec<Point2> {
        let should_use_low_gas_bases = self.state.desired_bases > 2;
        let mut expansions: Vec<_> = self
            .expansions
            .iter()
            .filter(|e| e.base.is_none() && (should_use_low_gas_bases || e.geysers.len() >= 2))
            .map(|e| e.loc)
            .collect();
        expansions.sort_by_key(|e| float_ord::FloatOrd(e.distance(self.start_location)));
        expansions
    }

    fn take_expansion(&mut self, position: Point2) -> bool {
        // FIXME: This may queue multiple hatcheries in the same expansion (:
        if self.can_afford(UnitTypeId::Hatchery, false) {
            println!("Expanding");
            self.create_building(UnitTypeId::Hatchery, position, true)
        } else {
            false
        }
    }

    pub fn get_rally_point(&self) -> Point2 {
        if let Some(pos) = self.state.bases.iter().closest(self.enemy_start) {
            pos.towards(self.enemy_start, 7.0)
        } else {
            self.start_location
        }
    }

    pub fn research_upgrade(
        &mut self,
        researcher: UnitTypeId,
        upgrade: UpgradeId,
        ability: AbilityId,
    ) -> bool {
        let researchers = self
            .units
            .my
            .all
            .filter(|unit| unit.type_id() == researcher && unit.is_ready() && unit.orders().len() < 5);
        if researchers.len() > 0
            && !self.has_upgrade(upgrade)
            && !self.is_ordered_upgrade(upgrade)
        {
            if let Some(candidate) = researchers.min(|unit| unit.orders().len()) {
                candidate.use_ability(ability, true);
            }
            true
        } else {
            false
        }
    }

    fn should_expand(&self) -> bool {
        let num_hatcheries = self.units.my.townhalls.len() + self.counter().ordered().count(UnitTypeId::Hatchery);
        let desired_bases = if self.runtime_options.use_tryhard_mining && self.supply_used > 34 {
            self.state.desired_bases.max(3)
        } else {
            self.state.desired_bases
        };
        self.supply_used >= self.state.build_order.first_hatch_supply
            && num_hatcheries < desired_bases
            && !self.state.is_under_attack
    }

    pub fn perform_building(&mut self, _iteration: usize) -> SC2Result<bool> {
        let mut did_attempt_build = true;
        // FIXME: This is ugly
        if self.state.desired_bases > 2 {
            self.state.desired_gasses = 8;
            self.state.desired_workers = 72;
        } else if self.state.desired_gasses == 2 && self.minerals >= 400 && self.vespene < 100 {
            self.state.desired_gasses = 4;
            self.state.desired_workers = 44;
        }
        let main_build_location = self.start_location.towards(self.game_info.map_center, 7.0);
        if (self.supply_used >= self.state.build_order.spawning_pool_supply || self.state.is_under_attack)
            && self.count_unit(UnitTypeId::SpawningPool) < 1
        {
            self.create_building(UnitTypeId::SpawningPool, main_build_location, false);
        } else if self.should_expand() {
            for expansion in self.determine_best_expansion_order() {
                if self.take_expansion(expansion) {
                    break;
                }
            }
        } else if self.supply_used >= 17 && self.ensure_taken_gasses(self.state.desired_gasses) {
            // Nothing to do here, ensure_taken_gasses does everything as a side effect
        } else if self.supply_used >= 32 && self.count_unit(UnitTypeId::RoachWarren) < 1 {
            self.create_building(UnitTypeId::RoachWarren, main_build_location, false);
        } else if self.units.my.townhalls.len() > 2 && self.state.desired_bases > 2 && self.count_unit(UnitTypeId::Lair) < 1 {
            if let Some(hatch) = self
                .units
                .my
                .townhalls
                .filter(|hatch| hatch.is_ready() && hatch.orders().is_empty())
                .first()
            {
                hatch.use_ability(AbilityId::UpgradeToLairLair, false);
            } else {
                did_attempt_build = false;
            }
        } else if self.counter().count(UnitTypeId::Lair) > 0
            && self.count_unit(UnitTypeId::HydraliskDen) < 1
        {
            // println!("{}: Want Hydra Den", _iteration);
            self.create_building(UnitTypeId::HydraliskDen, main_build_location, false);
        } else if (self.counter().count(UnitTypeId::Lair) > 0 && self.research_upgrade(
            UnitTypeId::RoachWarren,
            UpgradeId::GlialReconstitution,
            AbilityId::ResearchGlialRegeneration,
        )) || self.research_upgrade(
            UnitTypeId::HydraliskDen,
            UpgradeId::EvolveGroovedSpines,
            AbilityId::ResearchGroovedSpines) ||
            self.research_upgrade(
                UnitTypeId::HydraliskDen,
                UpgradeId::EvolveMuscularAugments,
                AbilityId::ResearchMuscularAugments,
            ) {
            did_attempt_build = true;
        } else {
            did_attempt_build = false;
        }
        Ok(did_attempt_build)
    }

    fn create_building(&mut self, unit_type: UnitTypeId, location: Point2, exact: bool) -> bool {
        let mut options = PlacementOptions::default();
        if exact {
            options.max_distance = 0;
        }
        if !self.can_afford(unit_type, false) {
            return false;
        }
        if let Some(w) = self
            .units
            .my
            .workers
            .filter(|u| !u.orders().iter().any(|o| o.ability.is_constructing()))
            .first()
        {
            if let Some(location) = self.find_placement(unit_type, location, options) {
                self.state.micro.drones.insert(w.tag(), crate::micro::DroneTask::Construct { queued_at: self.current_iteration });
                w.build(unit_type, location, false);
                self.subtract_resources(unit_type, false);
                return true;
            }
        }
        false
    }

    pub fn perform_training(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_hatcheries = self.count_unit(UnitTypeId::Hatchery);
        let has_roachwarren = self.counter().count(UnitTypeId::RoachWarren) > 0;
        let has_hydraden = self.counter().count(UnitTypeId::HydraliskDen) > 0;
        let is_mineral_starved = self.minerals < 200 && self.vespene > 800;
        let is_gas_starved = self.vespene < 100 && self.minerals > 600;
        for l in self.units.my.larvas.idle() {
            let num_workers =
                self.supply_workers as usize + self.counter().ordered().count(UnitTypeId::Drone);
            if self.calculate_pending_supply()
                < std::cmp::min(self.supply_used as usize + 6 * num_hatcheries, 200)
            {
                if self.can_afford(UnitTypeId::Overlord, false) {
                    l.train(UnitTypeId::Overlord, false);
                }
            } else if (!self.state.is_under_attack)
                && num_workers < self.state.desired_workers
                && self.can_afford(UnitTypeId::Drone, true)
            {
                l.train(UnitTypeId::Drone, false);
            } else if has_hydraden
                && (self.count_unit(UnitTypeId::Roach) > self.count_unit(UnitTypeId::Hydralisk) || is_mineral_starved)
            {
                l.train(UnitTypeId::Hydralisk, false);
            } else if has_roachwarren && self.can_afford(UnitTypeId::Roach, true) {
                l.train(UnitTypeId::Roach, false);
            } else if ((self.state.is_under_attack && !has_hydraden && !has_roachwarren) || is_gas_starved)
                && self.can_afford(UnitTypeId::Zergling, true)
            {
                l.train(UnitTypeId::Zergling, false);
            }
        }
        if self.count_unit(UnitTypeId::SpawningPool) > 0
            && self.count_unit(UnitTypeId::Queen) < self.units.my.townhalls.len()
        {
            if let Some(hatch) = self.least_busy_hatch() {
                hatch.train(UnitTypeId::Queen, true);
            }
        }
        Ok(())
    }
}
