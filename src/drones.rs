use rsbwapi::{PlayerId, Unit, UnitId, UnitType};
use std::{
    collections::{HashMap, HashSet},
    mem,
};

const PRINT_INTERVAL: i32 = 1_000; // frames

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub(crate) enum DroneRole {
    Minerals,
    Gas,
    Scout,
    Builder,
    Idle,
    // TODO: defending/attacking?
}

struct DroneAndRole {
    drone: Unit,
    role: DroneRole,
}

pub(crate) struct DroneManager {
    us: PlayerId,
    by_roles: HashMap<DroneRole, HashSet<UnitId>>,
    by_id: HashMap<UnitId, DroneAndRole>,
}

impl DroneManager {
    pub fn new() -> Self {
        DroneManager {
            us: PlayerId::default(),
            by_roles: HashMap::new(),
            by_id: HashMap::new(),
        }
    }

    pub fn on_unit_complete(&mut self, unit: Unit) {
        if unit.get_player().get_id() == self.us {
            let type_ = unit.get_type();
            match type_ {
                UnitType::Zerg_Drone => {
                    let id = unit.get_id();
                    self.by_id.insert(
                        unit.get_id(),
                        DroneAndRole {
                            drone: unit,
                            role: DroneRole::Idle,
                        },
                    );
                    self.add_to_role_map(id, DroneRole::Idle);
                }
                // if it's a complete building then it's not a drone anymore
                _ if type_.is_building() => {
                    self.on_unit_destroy(unit.get_id());
                }
                _ => {}
            }
        }
    }

    pub fn on_unit_destroy(&mut self, id: UnitId) {
        if let Some(dar) = self.by_id.remove(&id) {
            self.by_roles.entry(dar.role).and_modify(|hm| {
                hm.remove(&id);
            });
        }
    }

    pub fn grab_and_assign(&mut self, role: DroneRole) -> Option<Unit> {
        let idle = self.grab_and_assign_from(role.clone(), DroneRole::Idle);
        if idle.is_some() {
            return idle;
        }
        match role {
            DroneRole::Minerals => None,
            _ => {
                println!("no idle drones, assigning one from minerals to {:?}", role);
                self.grab_and_assign_from(role, DroneRole::Minerals)
            }
        }
    }

    fn grab_and_assign_from(&mut self, role: DroneRole, from_role: DroneRole) -> Option<Unit> {
        let drone = self
            .by_roles
            .get(&from_role)
            .map(|hs| hs.iter().next())
            .flatten()
            .map(|id| self.by_id.get(id))
            .flatten()
            .map(|d| &d.drone);
        match drone {
            Some(drone) => {
                let cloned = drone.clone();
                self.reassign(drone.get_id(), role);
                return Some(cloned);
            }
            None => None,
        }
    }

    pub fn idle(&mut self, id: UnitId) {
        if self.by_id.contains_key(&id) {
            self.reassign(id, DroneRole::Idle);
        }
    }

    fn reassign(&mut self, id: UnitId, role: DroneRole) {
        self.by_id.entry(id).and_modify(|dar| {
            let mut role = role.clone();
            mem::swap(&mut dar.role, &mut role);
            // role now contains the old role that we need to remove
            self.by_roles.entry(role).and_modify(|hm| {
                hm.remove(&id);
            });
        });
        self.add_to_role_map(id, role);
    }

    fn add_to_role_map(&mut self, id: UnitId, role: DroneRole) {
        self.by_roles
            .entry(role.clone())
            .or_insert_with(HashSet::new)
            .insert(id);
    }

    pub fn count_role(&self, role: &DroneRole) -> usize {
        self.by_roles.get(role).map_or(0, HashSet::len)
    }

    pub fn print_stats(&self, frame: i32) {
        if frame % PRINT_INTERVAL == 0 {
            let counts: Vec<_> = self.by_roles.iter().map(|(r, hm)| (r, hm.len())).collect();
            println!(
                "frame {} :: {} drones, {:?}",
                frame,
                self.by_id.len(),
                counts
            );
        }
    }

    pub fn set_player(&mut self, us: PlayerId) {
        self.us = us;
    }
}
