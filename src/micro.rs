use rust_sc2::prelude::*;

use crate::state::{BotState, GetBotState};
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
        if let Some(unit) = self.units.enemy.all.first() {
            return unit.position();
        }
        for point in self.state.micro.base_locations_by_expansion_order.iter() {
            if self.is_hidden(*point) {
                println!("Point {:?} is fogged", *point);
                return *point;
            }
        }
        self.enemy_start
    }
    pub fn perform_micro(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_roaches = self.counter().count(UnitTypeId::Roach);
        if num_roaches > self.state.peak_roaches {
            println!("A moving with {} roaches", num_roaches);
            let army = &self.units.my.units.of_type(UnitTypeId::Roach).idle();
            let target = self.determine_most_important_target();
            self.a_move(army, target, false);
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
}