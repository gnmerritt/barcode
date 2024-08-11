use rsbwapi::*;

pub struct BotCallbacks;

impl AiModule for BotCallbacks {
    fn on_frame(&mut self, game: &Game) {
        let self_ = game.self_().unwrap();
        let frame_minerals = self_.minerals();
        let _frame_gas = self_.gas();
        let my_units = self_.get_units();
        let idle_workers = my_units.iter().filter(|u| u.get_type() == UnitType::Zerg_Drone && u.is_idle());
        for w in idle_workers {
            let patch = w.get_closest_unit(|u|u.get_type().is_mineral_field(), 20);
            if let Some(patch) = patch {
                 w.gather(&patch).ok();
            }
        }
        if self_.supply_used() >= self_.supply_total() - 1 {
            let larva = my_units.iter()
                .find(|u| u.get_type() == UnitType::Zerg_Larva);
            if let Some(larva ) = larva {
                if frame_minerals > 100 {
                    larva.train(UnitType::Zerg_Overlord).ok();
                }
            }
        }
    }
}

fn main() {
    rsbwapi::start(|_game| BotCallbacks);
}
