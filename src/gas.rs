use rsbwapi::*;
use std::collections::{HashMap, HashSet};

use crate::drones::{DroneManager, DroneRole};

const DRONES_PER_GAS: usize = 3;

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

    fn ensure_mining(&mut self, drones: &mut DroneManager) {
        let living_drones: HashSet<_> = self
            .drones
            .iter()
            .filter(|u| u.exists() && u.get_type() == UnitType::Zerg_Drone)
            .map(|u| u.get_id())
            .collect();
        let needed_drones = DRONES_PER_GAS - living_drones.len();
        if needed_drones == 0 {
            return;
        }
        println!("gas {:?} needs {} more drones", self.gas, needed_drones);

        for _ in 0..needed_drones {
            if let Some(d) = drones.grab_and_assign(DroneRole::Gas) {
                let res = d.gather(&self.gas);
                match res {
                    Ok(true) => self.drones.push(d),
                    _ => println!("couldn't mine gas with drone {}: {:?}", d.get_id(), res),
                }
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

    pub fn on_frame(&mut self, game: &Game, drones: &mut DroneManager) {
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

            for mut mg in mining_gasses {
                mg.ensure_mining(drones);
                self.gasses.insert(mg.gas.get_id(), mg);
            }
        }
    }
}
