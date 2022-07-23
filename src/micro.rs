use rust_sc2::prelude::*;

use crate::bot::FaxBot;

#[derive(Default, Debug)]
pub struct MicroState {
    pub enemy_base_locations_by_expansion_order: Vec<Point2>,
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
        for point in self
            .state
            .micro
            .enemy_base_locations_by_expansion_order
            .iter()
        {
            if self.is_hidden(*point) {
                return *point;
            }
        }
        self.state.map_info.get_random_point()
    }
    pub fn perform_micro(&mut self, _iteration: usize) -> SC2Result<()> {
        let mut army_types = vec![UnitTypeId::Roach, UnitTypeId::Hydralisk];
        let army_count =
            self.counter().count(UnitTypeId::Roach) + self.counter().count(UnitTypeId::Hydralisk);
        if self.state.is_under_attack
            || army_count > self.state.peak_army
            || self.supply_used >= 150
        {
            if self.state.is_under_attack {
                army_types.push(UnitTypeId::Zergling);
            }
            let active_army_count = self
                .units
                .my
                .units
                .filter(|u| army_types.contains(&u.type_id()))
                .len();
            let army = &mut self
                .units
                .my
                .units
                .filter(|u| army_types.contains(&u.type_id()))
                .idle();
            let target = self.determine_most_important_target();
            if !army.is_empty() {
                println!(
                    "A-moving {}/{} units to {:?}",
                    army.len(),
                    active_army_count,
                    target
                );
                self.a_move(army, target, false);
            }
            self.state.peak_army = army_count;
        }
        {
            self.position_queens();
            let idle_hatcheries = self
                .units
                .my
                .townhalls
                .filter(|hatch| !hatch.buffs().contains(&BuffId::QueenSpawnLarvaTimer));
            let ready_queens = self.units.my.units.idle().filter(|q| {
                q.type_id() == UnitTypeId::Queen
                    && q.energy().unwrap() as usize
                        >= self.energy_cost(AbilityId::EffectInjectLarva).unwrap()
            });
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
        let mut queens = self
            .units
            .my
            .units
            .filter(|u| u.type_id() == UnitTypeId::Queen)
            .idle();
        for hatch in self.units.my.townhalls.iter() {
            if let Some(nearest_queen) = queens
                .filter(|u| u.distance(hatch) < 8.0)
                .closest(hatch)
                .map(|u| u.tag())
            {
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
            match assigned.cmp(&ideal) {
                std::cmp::Ordering::Less => {
                    for _ in 0..(ideal - assigned) {
                        undermined_resources.push(gas.tag());
                    }
                }
                std::cmp::Ordering::Greater => {
                    let nearby_miners = self
                        .units
                        .my
                        .workers
                        .filter(|u| u.distance(gas) < 8.0 && u.target_tag() == Some(gas.tag()));
                    surplus_workers.extend(
                        nearby_miners
                            .iter()
                            .take((assigned - ideal) as usize)
                            .cloned(),
                    );
                }
                _ => (),
            }
        }
        for base in self.units.my.townhalls.ready() {
            let assigned = base.assigned_harvesters().unwrap();
            let ideal = base.ideal_harvesters().unwrap();
            match assigned.cmp(&ideal) {
                std::cmp::Ordering::Less => {
                    let mineral_patch = self.units.mineral_fields.closest(base.position()).unwrap();
                    for _ in 0..(ideal - assigned) {
                        undermined_resources.push(mineral_patch.tag());
                    }
                }
                std::cmp::Ordering::Greater => {
                    let local_minerals = self
                        .units
                        .mineral_fields
                        .iter()
                        .closer(11.0, base.position())
                        .map(|u| u.tag())
                        .collect::<Vec<_>>();
                    let nearby_miners = self.units.my.workers.filter(|u| {
                        u.target_tag().map_or(false, |tag| {
                            local_minerals.contains(&tag)
                                || (u.is_carrying_minerals() && tag == base.tag())
                        })
                    });
                    surplus_workers.extend(
                        nearby_miners
                            .iter()
                            .take((assigned - ideal) as usize)
                            .cloned(),
                    );
                }
                _ => (),
            }
        }
        for (worker, resource) in surplus_workers.iter().zip(undermined_resources.iter()) {
            worker.gather(*resource, false);
        }
        Ok(())
    }
}
