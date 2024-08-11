use rsbwapi::*;

pub struct BotCallbacks;

fn spawn_maybe(units: &Vec<Unit>, utype: UnitType) {
    println!("trying to spawn a {:?}", utype);
    let larva = units.iter().find(|u| u.get_type() == UnitType::Zerg_Larva);
    if let Some(larva) = larva {
        larva.train(utype).ok();
    }
}

impl AiModule for BotCallbacks {
    fn on_frame(&mut self, game: &Game) {
        let self_ = game.self_().unwrap();
        let frame_minerals = self_.minerals();
        let _frame_gas = self_.gas();
        let my_units = self_.get_units();
        let idle_workers: Vec<&Unit> = my_units
            .iter()
            .filter(|u| u.get_type() == UnitType::Zerg_Drone && u.is_idle()).collect();
        println!("got {} idle workers", idle_workers.len());
        for w in idle_workers {
            let patch = w.get_closest_unit(|u| u.get_type().is_mineral_field(), 100);
            if let Some(patch) = patch {
                w.gather(&patch).ok();
            }
        }
        if self_.supply_used() >= self_.supply_total() - 1 && frame_minerals > 100 {
            spawn_maybe(&my_units, UnitType::Zerg_Overlord);
        }
        if frame_minerals > 50 {
            spawn_maybe(&my_units, UnitType::Zerg_Larva);
        }
    }
}

fn main() {
    rsbwapi::start(|_game| BotCallbacks);
}
