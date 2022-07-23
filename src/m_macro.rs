use rust_sc2::prelude::*;

use crate::bot::FaxBot;

impl FaxBot {
    /// Returns whether or not gasses were taken this iteration
    fn ensure_taken_gasses(&mut self, num_gasses: usize) -> bool {
        if self.count_unit(UnitTypeId::Extractor) >= self.state.desired_gasses {
            return false;
        }
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
        for base in self.state.bases.iter() {
            if self.can_afford(UnitTypeId::Extractor, false) && current_gasses < num_gasses {
                if let Some(nearest_free_gas) = self.find_gas_placement(*base) {
                    if let Some(w) = self.units.my.workers.first() {
                        w.build_gas(nearest_free_gas.tag(), false);
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
    ) {
        if self.count_unit(researcher) > 0
            && !self.has_upgrade(upgrade)
            && !self.is_ordered_upgrade(upgrade)
            && self.can_afford_upgrade(upgrade)
        {
            let researchers = self
                .units
                .my
                .all
                .filter(|unit| unit.is_ready() && unit.orders().len() < 5);
            if let Some(candidate) = researchers.min(|unit| unit.orders().len()) {
                candidate.use_ability(ability, true);
            }
        }
    }

    fn should_expand(&self) -> bool {
        let num_hatcheries = self.count_unit(UnitTypeId::Hatchery);
        self.supply_used >= 17
            && num_hatcheries < self.state.desired_bases
            && !self.state.is_under_attack
    }

    pub fn perform_building(&mut self, _iteration: usize) -> SC2Result<bool> {
        let mut did_attempt_build = true;
        // FIXME: This is ugly
        if self.state.desired_bases > 2 {
            self.state.desired_gasses = 6;
            self.state.desired_workers = 56;
        } else if self.state.desired_gasses == 2 && self.minerals >= 400 && self.vespene < 100 {
            self.state.desired_gasses = 4;
            self.state.desired_workers = 44;
        }
        let main_base = self.start_location.towards(self.game_info.map_center, 5.0);
        if self.supply_used >= 17 && self.count_unit(UnitTypeId::SpawningPool) < 1 {
            // println!("{}: Want Spawning pool", _iteration);
            self.create_building(UnitTypeId::SpawningPool, main_base, false);
        } else if self.should_expand() {
            // println!("{}: Want Expand", _iteration);
            for expansion in self.determine_best_expansion_order() {
                if self.take_expansion(expansion) {
                    break;
                }
            }
        } else if self.supply_used >= 17 && self.ensure_taken_gasses(self.state.desired_gasses) {
            // println!("{}: Want Extractor", _iteration);
        } else if self.supply_used >= 32 && self.count_unit(UnitTypeId::RoachWarren) < 1 {
            // println!("{}: Want Roach Warren", _iteration);
            self.create_building(UnitTypeId::RoachWarren, main_base, false);
        } else if self.units.my.townhalls.len() > 2 && self.count_unit(UnitTypeId::Lair) < 1 {
            // println!("{}: Want Lair", _iteration);
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
            self.create_building(UnitTypeId::HydraliskDen, main_base, false);
        } else {
            did_attempt_build = false;
        }
        self.research_upgrade(
            UnitTypeId::RoachWarren,
            UpgradeId::GlialReconstitution,
            AbilityId::ResearchGlialRegeneration,
        );
        self.research_upgrade(
            UnitTypeId::HydraliskDen,
            UpgradeId::EvolveGroovedSpines,
            AbilityId::ResearchGroovedSpines,
        );
        self.research_upgrade(
            UnitTypeId::HydraliskDen,
            UpgradeId::EvolveMuscularAugments,
            AbilityId::ResearchMuscularAugments,
        );
        Ok(did_attempt_build)
    }

    fn create_building(&mut self, unit_type: UnitTypeId, location: Point2, exact: bool) -> bool {
        let mut options = PlacementOptions::default();
        if exact {
            options.max_distance = 0;
        }
        if let Some(w) = self.units.my.workers.first() {
            if let Some(location) = self.find_placement(unit_type, location, options) {
                w.build(unit_type, location, false);
                return true;
            }
        }
        false
    }

    pub fn perform_training(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_hatcheries = self.count_unit(UnitTypeId::Hatchery);
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
            } else if self.count_unit(UnitTypeId::HydraliskDen) > 0
                && self.count_unit(UnitTypeId::Roach) > self.count_unit(UnitTypeId::Hydralisk)
            {
                l.train(UnitTypeId::Hydralisk, false);
            } else if self.count_unit(UnitTypeId::RoachWarren) > 0
                && self.can_afford(UnitTypeId::Roach, true)
            {
                l.train(UnitTypeId::Roach, false);
            } else if self.state.is_under_attack && self.can_afford(UnitTypeId::Zergling, true) {
                l.train(UnitTypeId::Zergling, false);
            }
        }
        if self.count_unit(UnitTypeId::SpawningPool) > 0
            && self.count_unit(UnitTypeId::Queen) < self.state.desired_bases
        {
            if let Some(hatch) = self.least_busy_hatch() {
                hatch.train(UnitTypeId::Queen, true);
            }
        }
        Ok(())
    }
}
