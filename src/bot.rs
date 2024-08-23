use crate::{
    build_order::BuildOrder,
    build_position::position_building,
    counts::Counts,
    drones::{DroneManager, DroneRole},
    gas::GasManager,
    scouting::Scout,
    seen::HaveSeen,
    unit_comp::UnitComp,
};
use rsbwapi::*;

pub struct BotCallbacks {
    build: BuildOrder,
    gasses: GasManager,
    drones: DroneManager,
    drone_scout: Option<Scout>,
    drone_builder: Option<Unit>,
    seen: HaveSeen,
    counts: Counts,
    enemy_race: Race,
}

impl BotCallbacks {
    pub fn new() -> Self {
        BotCallbacks {
            build: BuildOrder::new(),
            gasses: GasManager::new(),
            seen: HaveSeen::new(),
            counts: Counts::new_fake(0),
            drones: DroneManager::new(),
            drone_scout: None,
            drone_builder: None,
            enemy_race: Race::Random,
        }
    }
}

impl BotCallbacks {
    fn set_enemy_race(&mut self, game: &Game) {
        if let Some(enemy) = game.enemy() {
            self.enemy_race = enemy.get_race();
            println!("playing against {:?}", self.enemy_race);
        }
    }

    fn build_new_building(&mut self, game: &Game, to_build: UnitType) {
        if let Some(drone) = self.drone_builder.as_ref() {
            if drone.is_morphing() {
                self.drone_builder = None;
            }
        }
        if self.drone_builder.is_none() {
            self.drone_builder = self.drones.grab_and_assign(DroneRole::Builder);
            println!("after grabbing a builder drone: {:?}", self.drone_builder);
        }

        if let Some(builder_drone) = self.drone_builder.as_ref() {
            if let Some(tp) = position_building(game, to_build, builder_drone, &self.seen) {
                game.draw_box_map(
                    tp.to_position(),
                    (tp + to_build.tile_size()).to_position(),
                    Color::White,
                    false,
                );
                if self.counts.can_afford(to_build) {
                    println!("placing a {:?} at {:?}", to_build, tp);

                    let res = builder_drone.build(to_build, tp);
                    if let Ok(true) = res {
                        self.build.placed_building(to_build);
                    } else {
                        println!(
                            "placing {:?} failed: {:?} - {:?}",
                            to_build,
                            res,
                            builder_drone.get_order()
                        );
                    }
                } else {
                    builder_drone
                        .move_((tp - to_build.tile_size()).to_position())
                        .ok();
                }
            }
        }
        // buildings spend when they start, don't spend the building's
        // money on units in the meantime
        self.counts.bought(to_build);
    }

    fn upgrade_building(&mut self, my_units: &Vec<Unit>, to_build: UnitType) {
        let predecessor = match to_build {
            UnitType::Zerg_Lair => UnitType::Zerg_Hatchery,
            UnitType::Zerg_Hive => UnitType::Zerg_Lair,
            UnitType::Zerg_Greater_Spire => UnitType::Zerg_Spire,
            _ => unreachable!(),
        };
        if let Some(to_upgrade) = my_units.iter().find(|u| u.get_type() == predecessor) {
            if let Ok(true) = to_upgrade.morph(to_build) {
                println!("morphed a {:?}", to_build);
                self.build.placed_building(to_build);
            }
            // set aside money for the upgrade whether it built or not
            self.counts.bought(to_build);
        }
    }

    fn send_drone_scount(&mut self, game: &Game) {
        let scout_timing = match self.enemy_race {
            Race::Random => 14,
            _ => 24,
        };
        if self.counts.supply_used() >= scout_timing && self.drone_scout.is_none() {
            let drone = self.drones.grab_and_assign(DroneRole::Scout);
            if let Some(drone) = drone {
                println!("assigned a drone scout {:?}", drone);
                drone.stop().ok();
                let mut scout = Scout::new(drone);
                for s in game.get_start_locations() {
                    scout.go_later(s);
                }
                self.drone_scout = Some(scout);
            }
        }
        if let Some(scout) = &mut self.drone_scout {
            if scout.is_alive() {
                scout.on_frame();
            } else {
                println!("need a new scout");
                self.drone_scout = None; // RIP drone, assign a new one
            }
        }
    }
}

impl AiModule for BotCallbacks {
    fn on_start(&mut self, game: &Game) {
        if let Some(self_) = game.self_() {
            self.drones.set_player(self_.get_id());
            for u in self_.get_units() {
                self.drones.on_unit_complete(u);
            }
        }
        self.set_enemy_race(game);
    }

    /// fires when e.g. a larva morphs into a drone
    fn on_unit_complete(&mut self, _game: &Game, unit: Unit) {
        self.drones.on_unit_complete(unit);
    }

    fn on_unit_discover(&mut self, game: &Game, unit: Unit) {
        self.seen.on_unit_discover(game, &unit);
    }

    fn on_unit_destroy(&mut self, _game: &Game, unit: Unit) {
        self.seen.on_unit_destroy(unit.get_id());
        self.drones.on_unit_destroy(unit.get_id());
    }

    fn on_frame(&mut self, game: &Game) {
        self.build.on_frame(game);
        self.seen.on_frame(game);
        self.counts = Counts::new(game, &self.build);
        self.drones.print_stats(self.counts.frame());
        self.gasses.on_frame(game, &mut self.drones);

        let self_ = game.self_().unwrap();
        let my_units = self_.get_units();

        // place our next building
        let next_building = self.build.get_next_building(&self.counts);
        match next_building {
            Some(
                to_build @ (UnitType::Zerg_Lair
                | UnitType::Zerg_Hive
                | UnitType::Zerg_Greater_Spire),
            ) => self.upgrade_building(&my_units, to_build),
            Some(to_build) if to_build.is_building() => self.build_new_building(game, to_build),
            Some(not_a_building) => {
                println!("saw non-building {:?} from build order", not_a_building)
            }
            None => {}
        }

        // assign idle workers to mine minerals
        // TODO: update this like the gas handler to balance drones
        let minerals = game.get_all_units().into_iter().filter(|u| {
            u.get_type().is_mineral_field() && u.is_visible() && !u.is_being_gathered()
        });
        for m in minerals {
            if let Some(worker) = self.drones.grab_and_assign(DroneRole::Minerals) {
                println!("worker {} gathering {:?}", worker.get_id(), &m);
                worker.gather(&m).ok();
            }
        }

        let comp = UnitComp::new(game);
        comp.spawn_units(game, &mut self.counts, &self.seen);

        // attack with idle zerglings and mutas
        let idle_army = my_units.iter().filter(|u| {
            (u.get_type() == UnitType::Zerg_Zergling || u.get_type() == UnitType::Zerg_Mutalisk)
                && u.is_idle()
        });
        // TODO: group up so we don't attack one by one
        let enemy = self.seen.get_enemy_building();
        let mut first = true;
        for army in idle_army {
            if let Some(enemy) = enemy {
                if first {
                    println!("attacking enemy {:?}", enemy);
                    first = false;
                }
                army.attack(enemy.position).ok();
            }
        }

        self.send_drone_scount(game);
    }
}

#[allow(dead_code)]
fn overlord_scout(game: &Game, self_: Player) {
    // scout other starting locs with overlords
    let hatch = self_
        .get_units()
        .into_iter()
        .find(|u| u.get_type() == UnitType::Zerg_Hatchery)
        .expect("dead when we have no hatcheries");
    let hatch_pos = hatch.get_tile_position();
    let mut overlords = self_
        .get_units()
        .into_iter()
        .filter(|u| u.get_type() == UnitType::Zerg_Overlord);
    for loc in game.get_start_locations() {
        if loc.distance(hatch_pos) > 20.0 {
            if let Some(overlord) = overlords.next() {
                overlord.move_(loc.to_position()).ok();
                //println!("sending overlord to scout {}", loc.to_position());
            }
        }
    }
}
