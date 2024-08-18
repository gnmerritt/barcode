use crate::{
    build_order::BuildOrder, build_position::position_building, counts::Counts, gas::GasManager,
    seen::HaveSeen, unit_comp::UnitComp,
};
use rsbwapi::*;
use std::collections::HashSet;

pub struct BotCallbacks {
    build: BuildOrder,
    gasses: GasManager,
    drone_scout_id: Option<UnitId>,
    drone_scout_locs: Vec<TilePosition>,
    seen: HaveSeen,
}

impl BotCallbacks {
    pub fn new() -> Self {
        BotCallbacks {
            build: BuildOrder::new(),
            gasses: GasManager::new(),
            seen: HaveSeen::new(),
            drone_scout_id: None,
            drone_scout_locs: vec![],
        }
    }
}

impl AiModule for BotCallbacks {
    fn on_unit_create(&mut self, _game: &Game, _unit: Unit) {
        // note: this seems to only fire for larva and not when they finish morphing
        /*
        println!(
            "{:?} created at {}",
            unit.get_type(),
            game.get_frame_count()
        );
        */
    }

    fn on_unit_destroy(&mut self, _game: &Game, unit: Unit) {
        self.seen.on_unit_destroy(unit.get_id());
    }

    fn on_frame(&mut self, game: &Game) {
        self.gasses.on_frame(game);
        self.build.on_frame(game);
        self.seen.on_frame(game);
        let mut counts = Counts::new(game, &self.build);

        let self_ = game.self_().unwrap();
        let my_units = self_.get_units();
        let mut used_drones = HashSet::new();

        // place our next building
        let next_building = self.build.get_next_building(&counts);
        if let Some(to_build) = next_building {
            if to_build == UnitType::Zerg_Lair || to_build == UnitType::Zerg_Hive {
                let base = if to_build == UnitType::Zerg_Lair {
                    UnitType::Zerg_Hatchery
                } else {
                    UnitType::Zerg_Lair
                };
                if let Some(to_upgrade) = my_units.iter().find(|u| u.get_type() == base) {
                    if let Ok(true) = to_upgrade.morph(to_build) {
                        println!("morphed a {:?}", to_build);
                        self.build.placed_building(to_build);
                    }
                }
            }
            // TODO this feels cludgy and will only get worse as we do more things
            let builder_drone = my_units.iter().find(|u| {
                u.get_type() == UnitType::Zerg_Drone
                    && !u.is_morphing()
                    && u.get_order() != Order::PlaceBuilding
                    && u.is_interruptible()
                    && Some(u.get_id()) != self.drone_scout_id
            });
            if let Some(builder_drone) = builder_drone {
                if let Some(tp) =
                    position_building(game, to_build, builder_drone, self.seen.get_gas_locs())
                {
                    used_drones.insert(builder_drone.get_id());
                    game.draw_box_map(
                        tp.to_position(),
                        (tp + to_build.tile_width()).to_position(),
                        Color::White,
                        false,
                    );
                    if counts.can_afford(to_build) {
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
                        builder_drone.move_(tp.to_position()).ok();
                    }
                }
            }
            // buildings spend when they start, don't spend the building's
            // money on units in the meantime
            counts.bought(to_build);
        }

        // assign idle workers to mine minerals
        let mut idle_workers = my_units.iter().filter(|u| {
            u.get_type() == UnitType::Zerg_Drone
                && u.is_idle()
                && !used_drones.contains(&u.get_id())
        });
        let minerals = game.get_all_units().into_iter().filter(|u| {
            u.get_type().is_mineral_field() && u.is_visible() && !u.is_being_gathered()
        });
        for m in minerals {
            if let Some(worker) = idle_workers.next() {
                println!("worker {} gathering {:?}", worker.get_id(), &m);
                worker.gather(&m).ok();
            }
        }

        let comp = UnitComp::new(game);
        comp.spawn_units(game, &mut counts, &self.seen);

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

        // send out a drone to scout
        if counts.supply_used() >= 20 && self.drone_scout_id.is_none() {
            let drone = my_units.iter().find(|u| {
                u.get_type() == UnitType::Zerg_Drone
                    && !u.is_morphing()
                    && !used_drones.contains(&u.get_id())
            });
            if let Some(drone) = drone {
                println!("assigned a drone scout {:?}", drone);
                drone.stop().ok();
                self.drone_scout_id = Some(drone.get_id());
                for s in game.get_start_locations() {
                    self.drone_scout_locs.push(s);
                }
            }
        }
        if let Some(scout) = self.drone_scout_id.map(|id| game.get_unit(id)).flatten() {
            if scout.exists() && scout.get_type() == UnitType::Zerg_Drone {
                if scout.is_idle() {
                    if let Some(next_loc) = self.drone_scout_locs.pop() {
                        println!("sending scout {} to {}", scout.get_id(), next_loc);
                        scout.move_(next_loc.to_position()).ok();
                    }
                }
            } else {
                println!("need a new scout");
                self.drone_scout_id = None; // RIP drone, assign a new one
            }
        }
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
