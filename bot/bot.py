from sc2 import BotAI, Race, UnitTypeId
from random import choice

WORKER_CAP = 16

class CompetitiveBot(BotAI):
    NAME: str = "Fax"
    RACE: Race = Race.Zerg

    def count_building(self, building_type):
        return self.already_pending(building_type) + self.structures.filter(lambda s: s.type_id == building_type and s.is_ready).amount

    async def on_start(self):
        print("Game started")
        self.ling_wave_size = 18

    async def try_build(self, building_type):
        if self.can_afford(building_type):
            print(f"Morphing {building_type}")
            pos = self.start_location.towards(self.game_info.map_center, distance=5)
            placement = await self.find_placement(building_type, near=pos, placement_step=1)
            if placement:
                build_worker = self.workers.closest_to(placement)
                build_worker.build(building_type, placement)

    async def perform_micro(self, iteration):
        # Order idle lings to attack a random enemy position if more than ling_wave_size lings
        num_lings = self.units.filter(lambda u: u.is_ready and u.type_id == UnitTypeId.ZERGLING).amount
        if num_lings >= self.ling_wave_size:
            idle_lings = self.units.filter(lambda u: u.is_idle and u.is_ready and u.type_id == UnitTypeId.ZERGLING)
            for ling in idle_lings:
                ling.attack(choice(self.enemy_start_locations))
            self.ling_wave_size = min(100, self.ling_wave_size + 8)

    async def morph_larva(self, iteration):
        my_larva = self.larva
        if len(my_larva) > 0:
            if self.supply_left > 0 and self.workers.amount < WORKER_CAP and self.can_afford(UnitTypeId.DRONE):
                my_larva.random.train(UnitTypeId.DRONE)
            elif self.supply_left > 0 and self.can_afford(UnitTypeId.ZERGLING):
                # Create lings if we can
                print("Morphing ling")
                my_larva.random.train(UnitTypeId.ZERGLING)
            elif self.can_afford(UnitTypeId.OVERLORD):
                # Create overlord if no supply left
                print("Morphing overlord")
                my_larva.random.train(UnitTypeId.OVERLORD)

    async def perform_macro(self, iteration):
        # Create a spawning pool if one doesn't exist
        if self.count_building(UnitTypeId.SPAWNINGPOOL) == 0:
            await self.try_build(UnitTypeId.SPAWNINGPOOL)
            return
        if self.count_building(UnitTypeId.HATCHERY) < 3 and self.workers.amount >= WORKER_CAP:
            await self.try_build(UnitTypeId.HATCHERY)

    async def on_step(self, iteration):
        await self.perform_macro(iteration)
        await self.morph_larva(iteration)
        await self.perform_micro(iteration)

    def on_end(self, result):
        print("Game ended.")
