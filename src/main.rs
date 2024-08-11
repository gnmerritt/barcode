use std::collections::hash_set::HashSet;
use rsbwapi::*;

struct BotCallbacks {
    buildings_ordered: HashSet<UnitType>, // worker dispatched, hasn't started yet
}

impl BotCallbacks {
    fn new() -> Self {
        BotCallbacks { buildings_ordered: HashSet::new() }
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
        let frame_minerals = self_.minerals();
        let _frame_gas = self_.gas();
        let my_units = self_.get_units();        
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
        if self_.supply_used() >= self_.supply_total() - 1 && frame_minerals >= 100 {
            spawn_maybe(&my_units, UnitType::Zerg_Overlord);
        }
        if frame_minerals >= 50 {
            spawn_maybe(&my_units, UnitType::Zerg_Drone);
        }
        let hatch = self_
            .get_units()
            .into_iter()
            .find(|u| u.get_type() == UnitType::Zerg_Hatchery)
            .expect("dead when we have no hatcheries");
        let hatch_pos = hatch.get_tile_position();
        for loc in game.get_start_locations() {
            if loc.distance(hatch_pos) > 20.0 {
                let overlord =
                    game.get_closest_unit(loc.to_position(), |u: &Unit| u.get_type() == UnitType::Zerg_Overlord, 99999);
                if let Some(overlord) = overlord {
                    overlord.move_(loc.to_position()).ok();
                }
            }
        }
    }
}

fn main() {
    rsbwapi::start(|_game| BotCallbacks::new());
}
