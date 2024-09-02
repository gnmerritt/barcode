use rsbwapi::*;
use std::collections::HashMap;

use crate::{
    counts::Counts,
    drones::{DroneManager, DroneRole},
};

const MAX_DRONES_PER_GAS: i8 = 3;

struct MiningGas {
    drones: Vec<Unit>,
    gas: Unit,
}

impl MiningGas {
    fn new(gas: Unit) -> Self {
        MiningGas {
            gas,
            drones: vec![],
        }
    }

    fn ensure_mining(&mut self, game: &Game, drones: &mut DroneManager, mining_count: i8) {
        let living_drone_count = self
            .drones
            .iter()
            .filter(|u| u.exists() && u.get_type() == UnitType::Zerg_Drone)
            .count();

        let needed_drones = mining_count - living_drone_count as i8;
        match needed_drones {
            _ if needed_drones < 0 => {
                println!(
                    "gas {:?} has too many drones, releasing {}",
                    self.gas, needed_drones
                );
                if let Some(to_release) = self.drones.pop() {
                    if to_release.is_carrying_gas() {
                        to_release.return_cargo().ok();
                    } else {
                        to_release.stop().ok();
                    }
                    drones.idle(to_release.get_id());
                }
            }
            _ if needed_drones > 0 => {
                println!("gas {:?} needs {} more drones", self.gas, needed_drones);
                for _ in 0..needed_drones {
                    if let Some(id) = drones.grab_and_assign(DroneRole::Gas) {
                        if let Some(d) = game.get_unit(id) {
                            let res = d.gather(&self.gas);

                            match res {
                                Ok(true) => self.drones.push(d),
                                _ => println!(
                                    "couldn't mine gas with drone {}: {:?}",
                                    d.get_id(),
                                    res
                                ),
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        for d in self.drones.iter() {
            if d.is_idle() {
                d.gather(&self.gas).ok();
            }
        }
    }
}

pub(crate) struct GasManager {
    gasses: HashMap<usize, MiningGas>,
}

impl GasManager {
    pub fn new() -> Self {
        GasManager {
            gasses: HashMap::new(),
        }
    }

    fn get_drones_per_gas(&self, counts: &Counts, drones: &DroneManager) -> i8 {
        let on_mins = drones.count_role(&DroneRole::Minerals);
        if counts.gas() > 1_000 && counts.minerals() < 500 {
            if on_mins < 12 {
                return 1;
            }
            if on_mins < 24 {
                return 2;
            }
        }
        MAX_DRONES_PER_GAS
    }

    pub fn on_frame(&mut self, game: &Game, counts: &Counts, drones: &mut DroneManager) {
        if let Some(self_) = game.self_() {
            let mining_gasses: Vec<_> = self_
                .get_units()
                .into_iter()
                .filter(|u| u.get_type() == UnitType::Zerg_Extractor && !u.is_morphing())
                .map(|e| {
                    self.gasses
                        .remove(&e.get_id())
                        .unwrap_or_else(|| MiningGas::new(e))
                })
                .collect();
            self.gasses.clear();

            let dpg = self.get_drones_per_gas(counts, drones);
            for mut mg in mining_gasses {
                mg.ensure_mining(game, drones, dpg);
                self.gasses.insert(mg.gas.get_id(), mg);
            }
        }
    }
}
