use rust_sc2::prelude::*;

use crate::bot::FaxBot;

#[derive(Default)]
pub struct MicroState {
    pub base_locations_by_expansion_order: Vec<Point2>,
}

impl FaxBot {
    fn a_move(&self, units: &Units, position: Point2, queue: bool) {
        for unit in units {
            unit.attack(Target::Pos(position), queue);
        }
    }
    fn determine_most_important_target(&self) -> Point2 {
        if let Some(unit) = self.units.enemy.all.filter(|u| !u.is_flying()).first() {
            return unit.position();
        }
        for point in self.state.micro.base_locations_by_expansion_order.iter() {
            if self.is_hidden(*point) {
                return *point;
            }
        }
        self.enemy_start
    }
    pub fn perform_micro(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_roaches = self.counter().count(UnitTypeId::Roach);
        if num_roaches > self.state.peak_roaches || self.supply_used >= 180 {
            let army = &self.units.my.units.of_type(UnitTypeId::Roach).idle();
            let target = self.determine_most_important_target();
            self.a_move(army, target, false);
            self.state.peak_roaches = num_roaches;
        }
        {
            self.position_queens();
            let idle_hatcheries = self.units.my.townhalls.filter(
                |hatch| !hatch.buffs().contains(&BuffId::QueenSpawnLarvaTimer));
            let ready_queens = self.units.my.units.idle().filter(|q| q.type_id() == UnitTypeId::Queen
                && q.energy().unwrap() as usize >= self.energy_cost(AbilityId::EffectInjectLarva).unwrap()
            );
            for hatch in idle_hatcheries.iter() {
                if let Some(queen) = ready_queens.filter(|q| q.distance(hatch) <= 8.0).first() {
                    queen.command(AbilityId::EffectInjectLarva, Target::Tag(hatch.tag()), true);
                }
            }
        }
        self.allocate_workers()?;
        Ok(())
    }
    fn position_queens(&mut self) {
        let mut unqueened_hatches = vec![];
        let mut queens = self.units.my.units.filter(|u| u.type_id() == UnitTypeId::Queen).idle();
        for hatch in self.units.my.townhalls.iter() {
            if let Some(nearest_queen) = queens.filter(|u| u.distance(hatch) < 8.0).closest(hatch).map(|u| u.tag()) {
                queens.remove(nearest_queen);
            } else {
                unqueened_hatches.push(hatch);
            }
        }
        for hatch in unqueened_hatches.iter() {
            if let Some(queen) = queens.pop() {
                queen.move_to(Target::Pos(hatch.position()), false);
            }
        }
    }
    fn allocate_workers(&mut self) -> SC2Result<()> {
        let mut surplus_workers = Units::new();
        let mut undermined_resources = vec![];
        surplus_workers.extend(self.units.my.workers.idle());
        for gas in &self.units.my.gas_buildings {
            let assigned = gas.assigned_harvesters().unwrap();
            let ideal = gas.ideal_harvesters().unwrap();
            if assigned < ideal {
                for _ in 0..(ideal - assigned) {
                    undermined_resources.push(gas.tag());
                }
            } else if assigned > ideal {
                let nearby_miners = self.units.my.workers
                    .filter(|u| u.distance(gas) < 8.0 && u.target_tag() == Some(gas.tag()));
                surplus_workers.extend(nearby_miners.iter().take((assigned - ideal) as usize).cloned());
            }
        }
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
        for (worker, resource) in surplus_workers.iter().zip(undermined_resources.iter()) {
            worker.gather(*resource, false);
        }
        Ok(())
    }
}