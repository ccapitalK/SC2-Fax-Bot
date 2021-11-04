use rust_sc2::prelude::*;

#[bot]
#[derive(Default)]
struct FaxBot {
    peak_lings: usize,
}

const WORKER_CAP: usize = 16;

impl FaxBot {
    fn count_building(&self, building_id: UnitTypeId) -> usize {
        self.counter().count(building_id) + self.counter().ordered().count(building_id)
    }
    fn perform_building(&mut self, _iteration: usize) -> SC2Result<()> {
        let main_base = self.start_location.towards(self.game_info.map_center, 5.0);
        if self.count_building(UnitTypeId::SpawningPool) < 1 {
            if let Some(w) = self.units.my.workers.first() {
                if let Some(location) = self.find_placement(UnitTypeId::SpawningPool, main_base, Default::default()) {
                    w.build(UnitTypeId::SpawningPool, location, false);
                }
            }
            return Ok(());
        } else {
            let num_hatcheries = self.count_building(UnitTypeId::Hatchery);
            if num_hatcheries < 3 {
                println!("Need to create {} hatcheries", 3 - num_hatcheries);
                if let Some(w) = self.units.my.workers.first() {
                    if let Some(location) = self.find_placement(UnitTypeId::Hatchery, main_base, Default::default()) {
                        w.build(UnitTypeId::Hatchery, location, false);
                    }
                }
            }
        }
        Ok(())
    }
    fn perform_training(&mut self, _iteration: usize) -> SC2Result<()> {
        if let Some(l) = self.units.my.larvas.idle().first() {
            let num_workers = self.units.my.workers.len();
            if num_workers < WORKER_CAP && self.can_afford(UnitTypeId::Drone, true) {
                l.train(UnitTypeId::Drone, false);
            } else if self.can_afford(UnitTypeId::Zergling, true) {
                l.train(UnitTypeId::Zergling, false);
            } else if self.can_afford(UnitTypeId::Overlord, false) {
                l.train(UnitTypeId::Overlord, false);
            }
        }
        Ok(())
    }
    fn perform_micro(&mut self, _iteration: usize) -> SC2Result<()> {
        let num_lings = self.counter().count(UnitTypeId::Zergling);
        if num_lings > self.peak_lings {
            for unit in &self.units.my.units.of_type(UnitTypeId::Zergling) {
                unit.attack(Target::Pos(self.enemy_start), false);
            }
            self.peak_lings = num_lings;
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

fn main() -> SC2Result<()> {
    let mut bot = FaxBot::default();
    run_vs_computer(
        &mut bot,
        Computer::new(Race::Random, Difficulty::Medium, None),
        "AutomatonLE",
        {
            let mut options = LaunchOptions::default();
            options.save_replay_as = Some("LastRun.SC2Replay");
            options
        },
    )
}
