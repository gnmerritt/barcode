use std::{thread, time::Duration};

use crate::{
    build_order::BuildOrder,
    build_position::{cartesian_center, position_building, tile_position_towards},
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
    start_position: TilePosition,
    enemy_race: Race,
    enemy_id: PlayerId,
}

const ONE_TILE: TilePosition = TilePosition { x: 1, y: 1 };

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
            enemy_id: PlayerId::default(),
            start_position: TilePosition::default(),
        }
    }
}

impl BotCallbacks {
    fn set_enemy_race(&mut self, game: &Game) {
        if let Some(enemy) = game.enemy() {
            self.enemy_race = enemy.get_race();
            self.enemy_id = enemy.get_id();
            println!("playing against {:?}", self.enemy_race);
        }
    }

    fn set_start_pos(&mut self, game: &Game) {
        for p in game.get_start_locations() {
            if game.is_visible(p) {
                self.start_position = p;
                println!("Our starting position is {:?}", p);
                break;
            }
        }
    }

    fn build_new_building(&mut self, game: &Game, to_build: UnitType) {
        if self.drone_builder.is_none() {
            if let Some(drone_id) = self.drones.grab_and_assign(DroneRole::Builder) {
                self.drone_builder = game.get_unit(drone_id);
                println!("after grabbing a builder drone: {:?}", self.drone_builder);
            }
        }

        game.draw_text_screen((10, 10), &format!("building {:?}", to_build));

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
                        self.build
                            .placed_building(to_build, Some(builder_drone.clone()));
                        self.drone_builder = None;
                    } else {
                        println!(
                            "placing {:?} failed: {:?} - {:?}",
                            to_build,
                            res,
                            builder_drone.get_order()
                        );
                    }
                } else {
                    builder_drone.move_((tp + ONE_TILE).to_position()).ok();
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
                self.build.upgraded_building(to_upgrade.clone(), to_build);
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
        // TODO: this supply used number seems to be buggy (too high)
        if self.counts.supply_used() >= scout_timing && self.drone_scout.is_none() {
            let drone = self.drones.grab_and_assign(DroneRole::Scout);
            if let Some(drone_id) = drone {
                if let Some(drone) = game.get_unit(drone_id) {
                    println!("assigned a drone scout {:?}", drone);
                    drone.stop().ok();
                    let mut scout = Scout::new(drone);
                    for s in game.get_start_locations() {
                        if !game.is_visible(s) {
                            scout.go_later(s);
                        }
                    }
                    for g in self.seen.get_gas_locs() {
                        if !game.is_visible(*g) {
                            scout.go_later(*g);
                        }
                    }
                    self.drone_scout = Some(scout);
                }
            }
        }
        if let Some(scout) = &mut self.drone_scout {
            if scout.is_done() {
                if self.drones.count_role(&DroneRole::Scout) > 0 {
                    self.drones.idle(scout.get_id());
                }
            } else {
                if scout.is_alive() {
                    scout.on_frame(game);
                } else {
                    // TODO: keep scout's state with new drone
                    println!("need a new scout");
                    self.drone_scout = None; // RIP drone, assign a new one
                }
            }
        }
    }

    fn check_drones_attacked(&mut self, my_units: &Vec<Unit>) {
        let drones = my_units
            .iter()
            .filter(|u| u.get_type() == UnitType::Zerg_Drone && !u.is_morphing());
        for d in drones {
            if d.is_attacking() || d.is_starting_attack() {
                continue;
            }
            let mut run_away = d.is_under_storm() || d.is_irradiated();
            if !run_away && d.is_under_attack() {
                let enemy = d
                    .get_units_in_radius(64, |u: &Unit| u.get_player().get_id() == self.enemy_id)
                    .into_iter()
                    .next();
                if let Some(enemy) = enemy {
                    // println!("drone {} attacking back", d.get_id());
                    d.attack(&enemy).ok();
                    self.drones.assign(d.get_id(), DroneRole::Defending);
                } else {
                    run_away = true;
                }
            }
            if run_away {
                // TODO run away better
                // println!("drone {} running away", d.get_id());
                d.move_((d.get_tile_position() + ONE_TILE + ONE_TILE).to_position())
                    .ok();
                self.drones.assign(d.get_id(), DroneRole::Fleeing);
            }
        }
    }

    fn get_outermost_hatch(&self, my_units: &Vec<Unit>) -> Option<Unit> {
        let mut outermost_hatch: Vec<&Unit> = my_units
            .iter()
            .filter(|u| u.get_type().is_successor_of(UnitType::Zerg_Hatchery))
            .collect();
        outermost_hatch.sort_by_cached_key(|h| {
            -h.get_position()
                .get_approx_distance(self.start_position.to_position())
        });
        outermost_hatch.into_iter().next().map(|u| u.clone())
    }

    // attack the center of the enemy units
    // or if we have enough go attack their base
    // TODO this is a stopgap obviously
    fn do_attacking(&self, game: &Game, my_units: &Vec<Unit>) {
        // attack with zerglings and mutas
        let army: Vec<&Unit> = my_units
            .iter()
            .filter(|u| {
                u.get_type() == UnitType::Zerg_Zergling || u.get_type() == UnitType::Zerg_Mutalisk
            })
            .collect();
        let enemy_units = self.seen.get_enemy_army();
        let enemy_positions = enemy_units
            .iter()
            .map(|su| su.position.to_tile_position())
            .collect();
        let enemy = cartesian_center(&enemy_positions).or_else(|| {
            if army.len() > 8 {
                self.seen
                    .get_enemy_building()
                    .map(|su| su.position.to_tile_position())
            } else {
                None
            }
        });
        if let Some(enemy) = enemy {
            let mut first = true;
            for army in army {
                if first {
                    // println!("attacking enemy {:?}", enemy);
                    first = false;
                    game.draw_box_map(
                        enemy.to_position(),
                        (enemy + ONE_TILE).to_position(),
                        Color::Green,
                        false,
                    );
                }
                if army.is_idle() {
                    army.attack(enemy.to_position()).ok();
                }
            }
        } else {
            let outermost_hatch = self
                .get_outermost_hatch(&my_units)
                .expect("dead without any hatches");
            let rally = if let Some(enemy) = enemy {
                tile_position_towards(&outermost_hatch.get_tile_position(), 2, &enemy)
            } else {
                outermost_hatch.get_tile_position() - ONE_TILE
            };
            game.draw_box_map(
                rally.to_position(),
                (rally + ONE_TILE).to_position(),
                Color::Green,
                false,
            );

            for a in army {
                if a.is_idle() && a.get_position().chebyshev_distance(rally.to_position()) > 3 {
                    a.attack(rally.to_position()).ok();
                }
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
        self.set_start_pos(game);
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
        self.seen.on_frame(game);
        self.build.on_frame(game);
        self.counts = Counts::new(game, &self.build);
        self.drones.on_frame(game);
        self.drones.print_stats(self.counts.frame());
        self.gasses.on_frame(game, &self.counts, &mut self.drones);
        self.build.release_drones(&mut self.drones);

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
            let worker_id = self.drones.grab_and_assign(DroneRole::Minerals);
            if let Some(worker) = worker_id.map(|id| game.get_unit(id)).flatten() {
                print!(
                    "frame {} :: worker {} gathering {:?}",
                    self.counts.frame(),
                    worker.get_id(),
                    &m
                );
                worker.gather(&m).ok();
            }
        }

        let mut comp = UnitComp::new(game);
        comp.spawn_units(game, &mut self.counts, &self.seen);

        self.do_attacking(game, &my_units);
        self.send_drone_scount(game);
        self.check_drones_attacked(&my_units);

        // makes games watchable
        if self.counts.supply_used() > 20 {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
