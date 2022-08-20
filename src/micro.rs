use rust_sc2::prelude::*;

use crate::bot::FaxBot;
use std::collections::{HashMap, HashSet};
use rust_sc2::units::Container;

fn count_distinct<T: Eq + std::hash::Hash, It>(it: It) -> HashMap<T, usize> where It: Iterator<Item=T> {
    let mut occurences = HashMap::new();
    for v in it {
        *occurences.entry(v).or_default() += 1;
    }
    occurences
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ResourceType {
    Gas,
    Mineral,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DroneTask {
    #[default]
    Idle,
    Construct {
        queued_at: usize,
    },
    Gather {
        resource_type: ResourceType,
        resource_tag: u64,
        hatch: u64,
    },
}

#[derive(Default, Debug)]
pub struct MicroState {
    pub enemy_base_locations_by_expansion_order: Vec<Point2>,
    // TODO: Should be a struct
    pub drones: HashMap<u64, DroneTask>,
    pub drone_last_seen: HashMap<u64, usize>,
}

impl MicroState {
    pub fn possibly_alive_drones(&self, iteration: usize) -> Vec<u64> {
        let mut possibly_alive = vec![];
        let gas_drone_timeout_in_ticks = 11 * 3;
        for (drone, task) in &self.drones {
            let last_seen = self.drone_last_seen[&drone];
            if let DroneTask::Gather { resource_type: ResourceType::Gas, .. } = task {
                if last_seen + gas_drone_timeout_in_ticks > iteration {
                    possibly_alive.push(*drone);
                }
            }
        }
        possibly_alive
    }
}

impl FaxBot {
    fn idle_drones(&self) -> Units {
        self.units.my.workers.filter(|u| self.state.micro.drones.get(&u.tag()) == Some(&DroneTask::Idle)).clone()
    }

    fn micro_drones(&mut self, iteration: usize) -> SC2Result<()> {
        let drones = self.units.my.workers.ready().clone();
        let mut alive_drones = self.units.my.workers.ready().tags().copied().collect::<HashSet<_>>();
        // XXX: The API doesn't show us any workers inside gas buildings
        alive_drones.extend(self.state.micro.possibly_alive_drones(iteration));
        let alive_drones = alive_drones;
        self.state.micro.drones.retain(|&k, _| alive_drones.contains(&k));
        for unit in drones {
            let tag = unit.tag();
            if !self.state.micro.drones.contains(&tag) {
                unit.stop(false);
                self.state.micro.drones.insert(tag, DroneTask::Idle);
            }
            self.state.micro.drone_last_seen.insert(tag, iteration);
        }
        self.allocate_workers(iteration)
    }

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

    fn move_overlords(&mut self, _iteration: usize) -> SC2Result<()> {
        if _iteration < 22 {
            let midpoint = self.state.map_info.midpoint();
            let overlords = self
                .units
                .my
                .units
                .filter(|u| u.type_id() == UnitTypeId::Overlord);
            for unit in overlords {
                unit.move_to(Target::Pos(midpoint), false);
            }
        }
        Ok(())
    }

    pub fn perform_micro(&mut self, iteration: usize) -> SC2Result<()> {
        self.move_overlords(iteration)?;
        let mut army_types = vec![
            UnitTypeId::Zergling,
            UnitTypeId::Roach,
            UnitTypeId::Hydralisk,
        ];
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
        self.micro_drones(iteration)?;
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
                queen.attack(Target::Pos(hatch.position()), false);
            }
        }
    }

    /// Returns vec of (hatch, resource) pairs
    fn get_relevant_resources(&self, units: Units) -> Vec<(Unit, Unit)> {
        assert!(self.units.my.townhalls.len() > 0);
        units.iter().filter_map(|u| {
            let townhalls = self.units.my.townhalls.ready();
            let closest = townhalls.closest(u.position()).unwrap();
            if closest.distance(u.position()) < 12.0 {
                Some((closest.clone(), u.clone()))
            } else {
                None
            }
        }).collect()
    }

    fn place_drones_on_resource(&mut self, hatch: Unit, resource: Unit, resource_type: ResourceType, currently_assigned: usize, max_assignable: usize) -> SC2Result<()> {
        let mut surplus_workers = self.idle_drones();
        let mut to_assign = vec![];
        let desired_for_resource = match resource_type {
            ResourceType::Mineral => 2usize,
            ResourceType::Gas => 3usize,
        };
        if currently_assigned < desired_for_resource {
            let to_add = desired_for_resource.saturating_sub(currently_assigned)
                .min(surplus_workers.len())
                .min(max_assignable);
            for _ in 0..to_add {
                let closest = surplus_workers.closest(resource.position()).unwrap();
                let task = DroneTask::Gather { hatch: hatch.tag(), resource_type, resource_tag: resource.tag() };
                to_assign.push((closest.tag(), task));
                surplus_workers.remove(closest.tag());
            }
        } else {
            let to_remove = currently_assigned - desired_for_resource;
            let workers_on_resource = self.state.micro.drones.iter().filter_map(|(&u, &task)| {
                match task {
                    DroneTask::Gather { resource_type: r, resource_tag, .. } if r == resource_type && resource_tag == resource.tag() => Some(u),
                    _ => None
                }
            });
            for worker in workers_on_resource.take(to_remove) {
                to_assign.push((worker, DroneTask::Idle));
            }
        }
        for (worker, task) in to_assign {
            self.state.micro.drones.insert(worker, task);
        }
        Ok(())
    }

    fn place_drones_on_resources(&mut self) -> SC2Result<()> {
        if self.units.my.townhalls.len() == 0 {
            return Ok(());
        }
        let currently_assigned = count_distinct(self
            .state
            .micro
            .drones
            .values()
            .filter_map(|e| if let DroneTask::Gather {
                resource_tag, ..
            } = e {
                Some(*resource_tag)
            } else {
                None
            }));
        let available_minerals = self.get_relevant_resources(self.units.mineral_fields.clone());
        let available_gasses = self.get_relevant_resources(self.units.my.gas_buildings.filter(|u| u.vespene_contents().unwrap() > 0).clone());
        let desired_mineral_workers = self.state.desired_workers - 3 * self.state.desired_gasses;
        let desired_gas_workers = self.units.my.workers.len().saturating_sub(desired_mineral_workers);
        let resource_something = [
            (available_minerals, desired_mineral_workers, ResourceType::Mineral, 2usize),
            (available_gasses, desired_gas_workers, ResourceType::Gas, 3usize),
        ];
        let mut desired_workers = 0usize;
        for (available_resource_patches, desired_workers_for_type, resource_type, desired_for_patch) in resource_something {
            desired_workers += desired_workers_for_type;
            for (hatch, resource) in available_resource_patches {
                if desired_workers == 0 {
                    break;
                }
                let currently_assigned = *currently_assigned.get(&resource.tag()).unwrap_or(&0);
                let max_assignable = desired_workers.saturating_sub(currently_assigned).min(desired_for_patch);
                self.place_drones_on_resource(hatch, resource, resource_type, currently_assigned, max_assignable)?;
                desired_workers = desired_workers.saturating_sub(desired_for_patch);
            }
        }
        Ok(())
    }

    fn calc_line(unit1: &Unit, unit2: &Unit) -> (Point2, Point2) {
        let point1 = unit1.position();
        let point2 = unit2.position();
        let rad1 = unit1.radius();
        let rad2 = unit2.radius();
        if point1.distance(point2) < rad1 + rad2 {
            let mid = Point2::new(point1.x + point2.x, point1.y + point2.y) / 2.0;
            (mid, mid)
        } else {
            let delta = point2 - point1;
            let dir = delta.normalize();
            (point1 + dir * rad1, point2 - dir * rad2)
        }
    }

    fn move_drones(&mut self, iteration: usize) -> SC2Result<()> {
        let should_tryhard_mine = self.runtime_options.use_tryhard_mining;
        let workers = self.units.my.workers.clone();
        let mut num_idle = 0usize;
        let mut num_construct = 0usize;
        for drone in workers {
            let task = *self.state.micro.drones.get(&drone.tag()).unwrap();
            let new_task = match task {
                DroneTask::Gather { hatch, resource_tag, resource_type } => {
                    let hatch = self.units.all.get(hatch);
                    let hatch_exists = hatch.is_some();
                    let resource_exists = if resource_type == ResourceType::Mineral {
                        self.units.mineral_fields.contains_tag(resource_tag)
                    } else {
                        self.units.my.gas_buildings.filter(|u| u.vespene_contents().unwrap() > 0).contains_tag(resource_tag)
                    };
                    if should_tryhard_mine {
                        if !(resource_exists && hatch_exists) {
                            drone.stop(false);
                            DroneTask::Idle
                        } else {
                            let resource = self.units.all.get(resource_tag).unwrap();
                            let hatch = hatch.unwrap();
                            let hatch_radius = hatch.radius();
                            let res_radius = resource.radius();
                            let path_dist = resource.position().distance(hatch.position());
                            let (hatch_near_pos, resource_near_pos) = Self::calc_line(hatch, resource);
                            let first_order = drone.order();
                            let check_current_order = |ability: AbilityId, target: Target| {
                                if let Some((a, t, _)) = first_order {
                                    a == ability && match (t, target) {
                                        (Target::Tag(t1), Target::Tag(t2)) => t1 == t2,
                                        (Target::Pos(t1), Target::Pos(t2)) => t1.distance(t2) <= 1.0,
                                        (Target::None, Target::None) => true,
                                        _ => false,
                                    }
                                } else {
                                    false
                                }
                            };
                            if drone.is_carrying_resource() {
                                let is_returning = check_current_order(AbilityId::HarvestReturnDrone, Target::Tag(hatch.tag()));
                                let distance = drone.position().distance(hatch.position());
                                if distance >= path_dist / 2.0 && !is_returning {
                                    if !is_returning {
                                        drone.return_resource(false);
                                    }
                                } else if distance > hatch_radius + 1.0 {
                                    if !check_current_order(AbilityId::Move, Target::Pos(hatch_near_pos)) {
                                        drone.move_to(Target::Pos(hatch_near_pos), false);
                                    }
                                } else if !is_returning {
                                    drone.return_resource(false);
                                }
                            } else {
                                let is_gathering = check_current_order(AbilityId::HarvestGatherDrone, Target::Tag(resource_tag));
                                let distance = drone.position().distance(resource.position());
                                if distance >= path_dist / 2.0 {
                                    if !is_gathering {
                                        drone.gather(resource_tag, false);
                                    }
                                } else if distance > res_radius + 1.0 {
                                    if !check_current_order(AbilityId::MoveMove, Target::Pos(resource_near_pos)) {
                                        drone.move_to(Target::Pos(resource_near_pos), false);
                                    }
                                } else if !is_gathering {
                                    drone.gather(resource_tag, false);
                                }
                            }
                            task
                        }
                    } else {
                        match drone.target() {
                            Target::Tag(targ) => {
                                if drone.is_returning() {
                                    task
                                } else if !(resource_exists && hatch_exists) {
                                    drone.stop(false);
                                    DroneTask::Idle
                                } else if targ != resource_tag {
                                    drone.gather(resource_tag, false);
                                    task
                                } else {
                                    task
                                }
                            }
                            // TODO: Position manually for tryhard mining
                            _ => {
                                if drone.is_carrying_resource() {
                                    drone.return_resource(false);
                                } else {
                                    drone.gather(resource_tag, false);
                                }
                                task
                            }
                        }
                    }
                }
                DroneTask::Construct { queued_at } => {
                    num_construct += 1;
                    if queued_at + 5 < iteration && !drone.orders().iter().any(|o| {
                        o.ability.is_constructing()
                    }) {
                        DroneTask::Idle
                    } else {
                        task
                    }
                }
                DroneTask::Idle => {
                    num_idle += 1;
                    if drone.order().is_some() {
                        drone.stop(false);
                    }
                    task
                }
                _ => task,
            };
            if new_task != task {
                self.state.micro.drones.insert(drone.tag(), new_task);
            }
        }
        Ok(())
    }

    fn allocate_workers(&mut self, iteration: usize) -> SC2Result<()> {
        self.place_drones_on_resources()?;
        self.move_drones(iteration)?;
        Ok(())
    }
}
