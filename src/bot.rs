use crate::{build_order::BuildOrder, build_position::position_building};
use rsbwapi::*;

pub struct BotCallbacks {
    build: BuildOrder,
}

impl BotCallbacks {
    pub fn new() -> Self {
        BotCallbacks {
            build: BuildOrder::new(),
        }
    }
}

fn spawn_maybe(units: &Vec<Unit>, utype: UnitType) {
    let larva = units.iter().find(|u| u.get_type() == UnitType::Zerg_Larva);
    if let Some(larva) = larva {
        println!("spawning a {:?}", utype);
        larva.train(utype).ok();
    }
}

impl AiModule for BotCallbacks {
    fn on_frame(&mut self, game: &Game) {
        let self_ = game.self_().unwrap();
        let mut frame_minerals = self_.minerals();
        let mut frame_gas = self_.gas();
        let my_units = self_.get_units();

        // place our next building
        if let Some(to_build) = self.build.get_next_building(game) {
            if frame_minerals >= to_build.mineral_price() && frame_gas >= to_build.gas_price() {
                let builder_drone = my_units
                    .iter()
                    .find(|u| u.get_type() == UnitType::Zerg_Drone && !u.is_idle());
                if let Some(builder_drone) = builder_drone {
                    if let Some((x, y)) = position_building(game, to_build, builder_drone) {
                        println!("placing a {:?} at ({},{})", to_build, x, y);
                        builder_drone.build(to_build, (x, y)).ok();
                        self.build.placed_building(to_build);
                        frame_minerals -= to_build.mineral_price();
                        frame_gas -= to_build.gas_price();
                    }
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
                println!("worker {:?} gathering {:?}", &worker, &m);
                worker.gather(&m).ok();
            }
        }

        // make overlords and drones
        // note: supply is doubled by BWAPI so that Zerglings can use an interger amount of supply
        if self_.supply_used() >= self_.supply_total() - 2 && frame_minerals >= 100 {
            spawn_maybe(&my_units, UnitType::Zerg_Overlord);
        }
        if frame_minerals >= 50 {
            spawn_maybe(&my_units, UnitType::Zerg_Drone);
        }

        // scout other starting locs with overlords
        let hatch = self_
            .get_units()
            .into_iter()
            .find(|u| u.get_type() == UnitType::Zerg_Hatchery)
            .expect("dead when we have no hatcheries");
        let hatch_pos = hatch.get_tile_position();
        println!("hatch at {hatch_pos}");
        let mut overlords = my_units
            .iter()
            .filter(|u| u.get_type() == UnitType::Zerg_Overlord);
        for loc in game.get_start_locations() {
            if loc.distance(hatch_pos) > 20.0 {
                if let Some(overlord) = overlords.next() {
                    overlord.move_(loc.to_position()).ok();
                    println!("sending overlord to scout {}", loc.to_position());
                }
            }
        }
    }
}
