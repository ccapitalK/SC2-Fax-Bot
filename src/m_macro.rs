use rust_sc2::prelude::*;

use crate::bot::FaxBot;

impl FaxBot {
    fn ensure_taken_gasses(&mut self, num_gasses: usize) {
        let current_gasses =
            self.units.my.structures
                .filter(|u| u.type_id() == UnitTypeId::Extractor && u.vespene_contents().map(|v| v > 0).unwrap_or(false)).len()
                + self.counter().ordered().count(UnitTypeId::Extractor);
        for base in self.state.bases.iter() {
            if self.can_afford(UnitTypeId::Extractor, false) && current_gasses < num_gasses {
                if let Some(nearest_free_gas) = self.find_gas_placement(*base) {
                    if let Some(w) = self.units.my.workers.first() {
                        w.build_gas(nearest_free_gas.tag(), false);
                        return;
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

    fn determine_best_expansion(&self) -> Option<Point2> {
        let should_use_low_gas_bases = self.state.desired_bases > 2;
        let expansions = self.expansions.iter()
            .filter(|e| e.base.is_none() && (should_use_low_gas_bases || e.geysers.len() >= 2))
            .map(|e| e.loc);
        expansions.closest(self.start_location)
    }

    fn take_expansion(&mut self, position: Point2) -> SC2Result<()> {
        if self.can_afford(UnitTypeId::Hatchery, false) {
            println!("Expanding");
            self.create_building(UnitTypeId::Hatchery, position);
        }
        Ok(())
    }

    pub fn get_rally_point(&self) -> Point2 {
        if let Some(pos) = self.state.bases.iter().closest(self.enemy_start) {
            pos.towards(self.enemy_start, 7.0)
        } else {
            self.start_location
        }
    }

    pub fn research_upgrade(&mut self, researcher: UnitTypeId, upgrade: UpgradeId, ability: AbilityId) {
        if self.count_unit(researcher) > 0 {
            if !self.has_upgrade(upgrade) && !self.is_ordered_upgrade(upgrade) && self.can_afford_upgrade(upgrade) {
                let researchers = self.units.my.all
                    .filter(|unit| unit.is_ready() && unit.orders().len() < 5);
                if let Some(candidate) = researchers.min(|unit| unit.orders().len()) {
                    candidate.use_ability(ability, true);
                }
            }
        }
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
        let num_hatcheries = self.count_unit(UnitTypeId::Hatchery);
        if self.supply_used >= 17 && self.count_unit(UnitTypeId::SpawningPool) < 1 {
            self.create_building(UnitTypeId::SpawningPool, main_base);
        } else if self.supply_used >= 17 && num_hatcheries < self.state.desired_bases {
            let expansion = self.determine_best_expansion().unwrap();
            self.take_expansion(expansion)?;
        } else if self.supply_used >= 17 && self.count_unit(UnitTypeId::Extractor) < self.state.desired_gasses {
            self.ensure_taken_gasses(self.state.desired_gasses);
        } else if self.supply_used >= 32 && self.count_unit(UnitTypeId::RoachWarren) < 1 {
            self.create_building(UnitTypeId::RoachWarren, main_base);
        } else if self.units.my.townhalls.len() > 2 && self.count_unit(UnitTypeId::Lair) < 1 {
            if let Some(hatch) = self.units.my.townhalls.filter(|hatch| hatch.is_ready() && hatch.orders().len() == 0).first() {
                hatch.use_ability(AbilityId::UpgradeToLairLair, false);
            } else {
                did_attempt_build = false;
            }
        } else if self.count_unit(UnitTypeId::Lair) > 0 && self.count_unit(UnitTypeId::HydraliskDen) < 1 {
            self.create_building(UnitTypeId::HydraliskDen, main_base);
        } else {
            did_attempt_build = false;
        }
        self.research_upgrade(UnitTypeId::RoachWarren, UpgradeId::GlialReconstitution, AbilityId::ResearchGlialRegeneration);
        self.research_upgrade(UnitTypeId::HydraliskDen, UpgradeId::EvolveGroovedSpines, AbilityId::ResearchGroovedSpines);
        self.research_upgrade(UnitTypeId::HydraliskDen, UpgradeId::EvolveMuscularAugments, AbilityId::ResearchMuscularAugments);
        Ok(did_attempt_build)
    }

    fn create_building(&mut self, unit_type: UnitTypeId, location: Point2) {
        if let Some(w) = self.units.my.workers.first() {
            if let Some(location) = self.find_placement(unit_type, location, Default::default()) {
                w.build(unit_type, location, false);
            }
        }
    }

    pub fn perform_training(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_hatcheries = self.count_unit(UnitTypeId::Hatchery);
        for l in self.units.my.larvas.idle() {
            let num_workers = self.supply_workers as usize + self.counter().ordered().count(UnitTypeId::Drone);
            if self.calculate_pending_supply() < std::cmp::min(self.supply_used as usize + 6 * num_hatcheries, 200) {
                if self.can_afford(UnitTypeId::Overlord, false) {
                    l.train(UnitTypeId::Overlord, false);
                }
            } else if (!self.state.is_under_attack) && num_workers < self.state.desired_workers && self.can_afford(UnitTypeId::Drone, true) {
                l.train(UnitTypeId::Drone, false);
            } else if self.count_unit(UnitTypeId::HydraliskDen) > 0
                && self.count_unit(UnitTypeId::Roach) > self.count_unit(UnitTypeId::Hydralisk) {
                l.train(UnitTypeId::Hydralisk, false);
            } else if self.count_unit(UnitTypeId::RoachWarren) > 0 && self.can_afford(UnitTypeId::Roach, true) {
                l.train(UnitTypeId::Roach, false);
            } else if self.state.is_under_attack && self.can_afford(UnitTypeId::Zergling, true) {
                l.train(UnitTypeId::Zergling, false);
            }
        }
        if self.count_unit(UnitTypeId::SpawningPool) > 0 && self.count_unit(UnitTypeId::Queen) < self.state.desired_bases {
            if let Some(hatch) = self.least_busy_hatch() {
                hatch.train(UnitTypeId::Queen, true);
            }
        }
        Ok(())
    }
}