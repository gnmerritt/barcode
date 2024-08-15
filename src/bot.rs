use crate::{
    build_order::BuildOrder, build_position::position_building, counts::Counts, gas::GasManager,
};
use rsbwapi::*;

pub struct BotCallbacks {
    build: BuildOrder,
    gasses: GasManager,
    drone_scout_id: Option<usize>,
}

impl BotCallbacks {
    pub fn new() -> Self {
        BotCallbacks {
            build: BuildOrder::new(),
            gasses: GasManager::new(),
            drone_scout_id: None,
        }
    }
}

fn spawn_maybe(units: &Vec<Unit>, utype: UnitType) -> Option<UnitType> {
    let larva = units.iter().find(|u| u.get_type() == UnitType::Zerg_Larva);
    if let Some(larva) = larva {
        if let Ok(true) = larva.train(utype) {
            println!("spawning a {:?}", utype);
            return Some(utype);
        }
    }
    None
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

    fn on_frame(&mut self, game: &Game) {
        self.build.check_placed_buildings(game);
        self.gasses.on_frame(game);
        let mut counts = Counts::new(game, &self.build);
        let self_ = game.self_().unwrap();
        let my_units = self_.get_units();

        // place our next building
        let next_building = self.build.get_next_building(self_.supply_used());
        if let Some(to_build) = next_building {
            if counts.can_afford(to_build) {
                let builder_drone = my_units
                    .iter()
                    .find(|u| u.get_type() == UnitType::Zerg_Drone && !u.is_idle());
                println!("found drone to build {:?}", to_build);
                if let Some(builder_drone) = builder_drone {
                    if let Some(tp) = position_building(game, to_build, builder_drone) {
                        println!("placing a {:?} at {:?}", to_build, tp);
                        let res = builder_drone.build(to_build, tp);
                        if let Ok(true) = res {
                            self.build.placed_building(to_build);
                        } else {
                            println!("placing {:?} failed: {:?}", to_build, res);
                            builder_drone.move_(tp.to_position()).ok();
                        }
                    }
                    // buildings spend when they start, don't spend the building's
                    // money on units in the meantime
                    counts.bought(to_build);
                }
            }
        }

        // assign idle workers to mine minerals
        let mut idle_workers = my_units
            .iter()
            .filter(|u| u.get_type() == UnitType::Zerg_Drone && u.is_idle());
        let minerals = game.get_all_units().into_iter().filter(|u| {
            u.get_type().is_mineral_field() && u.is_visible() && !u.is_being_gathered()
        });
        for m in minerals {
            if let Some(worker) = idle_workers.next() {
                println!("worker {} gathering {:?}", worker.get_id(), &m);
                worker.gather(&m).ok();
            }
        }

        // make overlords and drones
        // note: supply is doubled by BWAPI so that Zerglings can use an interger amount of supply
        if self_.supply_used() >= counts.supply_max() - 2
            && counts.can_afford(UnitType::Zerg_Overlord)
        {
            let morphing_overlord = my_units.iter().find(|u| {
                u.get_type() == UnitType::Zerg_Egg
                    && u.is_morphing()
                    && u.get_build_type() == UnitType::Zerg_Overlord
            });
            if morphing_overlord.is_none() {
                println!(
                    "need an overlord {} at {}",
                    counts.supply_string(),
                    counts.frame()
                );
                spawn_maybe(&my_units, UnitType::Zerg_Overlord).map(|u| counts.bought(u));
            }
        }
        if counts.can_afford(UnitType::Zerg_Drone) {
            spawn_maybe(&my_units, UnitType::Zerg_Drone).map(|u| counts.bought(u));
        }

        // send one drone to the center of the map to find our natural
        if self_.supply_used() >= 20 && self.drone_scout_id.is_none() {
            let drone = my_units
                .iter()
                .find(|u| u.get_type() == UnitType::Zerg_Drone);
            if let Some(drone) = drone {
                self.drone_scout_id = Some(drone.get_id());
                let x = game.map_width() / 2;
                let y = game.map_height() / 2;
                let tp = TilePosition { x, y };
                println!(
                    "sending drone scout to middle position={:?}",
                    tp.to_position()
                );
                drone.move_(tp.to_position()).ok();
            }
        }

        // scout other starting locs with overlords
        let hatch = self_
            .get_units()
            .into_iter()
            .find(|u| u.get_type() == UnitType::Zerg_Hatchery)
            .expect("dead when we have no hatcheries");
        let hatch_pos = hatch.get_tile_position();
        let mut overlords = my_units
            .iter()
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
}
