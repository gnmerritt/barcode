use crate::{
    build_position::get_hatches,
    counts::Counts,
    drones::{DroneManager, DroneRole},
};
use rsbwapi::{Game, Unit, UnitId, UnitType};
use std::collections::{HashMap, HashSet};

pub(crate) struct MineralManager {
    patch_to_drones: HashMap<UnitId, Vec<UnitId>>,
}

impl MineralManager {
    pub fn new() -> Self {
        MineralManager {
            patch_to_drones: HashMap::new(),
        }
    }

    pub fn on_frame(&mut self, game: &Game, counts: &Counts, drones: &mut DroneManager) {
        let mut patches = HashSet::new();
        for base in get_hatches(game) {
            let mid_of_base = base.get_tile_position() + base.get_type().tile_size() / 2;
            let mins = game
                .get_units_in_radius(mid_of_base.to_position(), 8 * 32, |u: &Unit| {
                    u.exists() && u.get_type().is_mineral_field()
                })
                .into_iter()
                .map(|m| m.get_id());
            patches.extend(mins);
        }

        let least_mined_patch = self
            .patch_to_drones
            .values()
            .map(|d| d.len())
            .min()
            .unwrap_or(0);

        for id in patches {
            if let Some(patch) = game.get_unit(id) {
                let mining = self.patch_to_drones.entry(id).or_insert_with(Vec::new);
                mining.retain(|drone_id| {
                    if let Some(drone) = game.get_unit(*drone_id) {
                        if drones.check_role(drone_id) != DroneRole::Minerals {
                            return false;
                        }
                        let is_mining = drone.exists()
                            && drone.get_type() == UnitType::Zerg_Drone
                            && !drone.is_morphing()
                            && !drone.is_idle();
                        if !is_mining {
                            drones.idle(*drone_id);
                        }
                        is_mining
                    } else {
                        false
                    }
                });
                if mining.len() > least_mined_patch {
                    continue;
                }
                if let Some(drone) = drones
                    .grab_and_assign(DroneRole::Minerals)
                    .map(|id| game.get_unit(id))
                    .flatten()
                {
                    let r = drone.gather(&patch);
                    if r.is_ok() {
                        mining.push(drone.get_id());
                        println!(
                            "frame {} :: worker {} gathering {:?}",
                            counts.frame(),
                            drone.get_id(),
                            &patch
                        );
                    } else {
                        println!(
                            "frame {} :: worker {} mining failed {:?}, ",
                            counts.frame(),
                            drone.get_id(),
                            r
                        );
                        drones.idle(drone.get_id());
                    }
                }
            } else {
                self.patch_to_drones.remove(&id);
            }
        }
    }
}
