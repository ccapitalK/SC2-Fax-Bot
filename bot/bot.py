from sc2 import BotAI, Race, UnitTypeId
from random import choice

WORKER_CAP = 16

class CompetitiveBot(BotAI):
    NAME: str = "CompetitiveBot"
    """Fax"""
    RACE: Race = Race.Zerg
    """This bot's Starcraft 2 race.
    Options are:
        Race.Terran
        Race.Zerg
        Race.Protoss
        Race.Random
    """

    def count_building(self, building_type):
        return self.already_pending(building_type) + self.structures.filter(lambda s: s.type_id == building_type and s.is_ready).amount

    async def on_start(self):
        print("Game started")
        # Do things here before the game starts

    async def try_build(self, building_type):
        if self.can_afford(building_type):
            print(f"Morphing {building_type}")
            pos = self.start_location.towards(self.game_info.map_center, distance=5)
            placement = await self.find_placement(building_type, near=pos, placement_step=1)
            if placement:
                build_worker = self.workers.closest_to(placement)
                build_worker.build(building_type, placement)

    async def on_step(self, iteration):
        # Create a spawning pool if one doesn't exist
        my_larva = self.larva
        if self.count_building(UnitTypeId.SPAWNINGPOOL) == 0:
            await self.try_build(UnitTypeId.SPAWNINGPOOL)
            return
        if self.count_building(UnitTypeId.HATCHERY) < 3 and self.workers.amount == WORKER_CAP:
            await self.try_build(UnitTypeId.HATCHERY)
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
        # Order idle lings to attack a random enemy position if more than 14 lings
        num_lings = self.units.filter(lambda u: u.is_ready and u.type_id == UnitTypeId.ZERGLING).amount
        if num_lings >= 18:
            idle_lings = self.units.filter(lambda u: u.is_idle and u.is_ready and u.type_id == UnitTypeId.ZERGLING)
            for ling in idle_lings:
                ling.attack(choice(self.enemy_start_locations))
        

    def on_end(self, result):
        print("Game ended.")
        # Do things here after the game ends
