use rsbwapi::{Game, PlayerId, Unit, UnitId, UnitType};
use std::collections::{HashMap, HashSet};

const PRINT_INTERVAL: i32 = 1_000; // frames

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub(crate) enum DroneRole {
    Minerals,
    Gas,
    Scout,
    Builder,
    Idle,
    Defending,
    Fleeing,
}

#[derive(Debug)]
pub(crate) struct DroneManager {
    us: PlayerId,
    by_roles: HashMap<DroneRole, HashSet<UnitId>>,
    by_id: HashMap<UnitId, DroneRole>,
}

impl DroneManager {
    pub fn new() -> Self {
        DroneManager {
            us: PlayerId::default(),
            // TODO: this can probably get simplified now
            by_roles: HashMap::new(),
            by_id: HashMap::new(),
        }
    }

    pub fn on_frame(&mut self, game: &Game) {
        // check for idle drones in temporary states and reassign them to
        // idle so they can get back to work
        let mut to_check: Vec<&usize> = vec![];
        if let Some(fleeing) = self.by_roles.get(&DroneRole::Fleeing) {
            to_check.extend(fleeing);
        }
        if let Some(defending) = self.by_roles.get(&DroneRole::Defending) {
            to_check.extend(defending);
        }

        let mut to_idle = vec![];
        for id in to_check {
            if let Some(drone) = game.get_unit(*id) {
                if drone.is_idle() {
                    to_idle.push(*id);
                }
            }
        }
        // TODO: there must be a way to call into mut self methods recursively
        for id in to_idle {
            self.idle(id);
        }
    }

    pub fn on_unit_complete(&mut self, unit: Unit) {
        if unit.get_player().get_id() == self.us {
            let type_ = unit.get_type();
            match type_ {
                UnitType::Zerg_Drone => self.insert(unit.get_id()),
                // if it's a complete building then it's not a drone anymore
                _ if type_.is_building() => {
                    self.on_unit_destroy(unit.get_id());
                }
                _ => {}
            }
        }
    }

    pub fn on_unit_destroy(&mut self, id: UnitId) {
        if let Some(role) = self.by_id.remove(&id) {
            self.by_roles.entry(role).and_modify(|hm| {
                hm.remove(&id);
            });
        }
    }

    pub fn grab_and_assign(&mut self, role: DroneRole) -> Option<UnitId> {
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

    fn grab_and_assign_from(&mut self, role: DroneRole, from_role: DroneRole) -> Option<UnitId> {
        let drone_id = self
            .by_roles
            .get(&from_role)
            .map(|hs| hs.iter().next())
            .flatten();
        match drone_id {
            Some(id) => {
                let cloned = id.clone();
                self.reassign(*id, role);
                return Some(cloned);
            }
            None => None,
        }
    }

    pub fn idle(&mut self, id: UnitId) {
        self.assign(id, DroneRole::Idle);
    }

    pub fn assign(&mut self, id: UnitId, role: DroneRole) {
        if self.by_id.contains_key(&id) {
            self.reassign(id, role);
        }
    }

    fn reassign(&mut self, id: UnitId, new_role: DroneRole) {
        let saved = new_role.clone();
        self.by_id.entry(id).and_modify(|role| {
            let old_role = role.clone();
            *role = new_role;
            self.by_roles.entry(old_role).and_modify(|hm| {
                hm.remove(&id);
            });
        });
        self.add_to_role_map(id, saved);
    }

    fn insert(&mut self, id: UnitId) {
        self.by_id.insert(id, DroneRole::Idle);
        self.add_to_role_map(id, DroneRole::Idle);
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

#[cfg(test)]
mod test {
    use super::{DroneManager, DroneRole};
    use rsbwapi::UnitId;

    #[test]
    fn test_lifecycle() {
        let mut d = DroneManager::new();
        assert_eq!(d.count_role(&DroneRole::Idle), 0, "no drones yet");

        let drone_id: UnitId = 7;
        d.insert(drone_id);
        assert_eq!(
            d.count_role(&DroneRole::Idle),
            1,
            "one idle drone after add"
        );

        let grabbed = d.grab_and_assign(DroneRole::Minerals);
        assert_eq!(grabbed, Some(drone_id), "got a drone");
        assert_eq!(
            d.count_role(&DroneRole::Minerals),
            1,
            "one mineral drone after grab and assign"
        );
        let none_left = d.grab_and_assign(DroneRole::Minerals);
        assert_eq!(none_left, None, "no idle drones left to grab");

        d.on_unit_destroy(drone_id);
        assert_eq!(
            d.count_role(&DroneRole::Minerals),
            0,
            "mineral drone got destroyed"
        );
        let still_none = d.grab_and_assign(DroneRole::Gas);
        assert_eq!(still_none, None, "no drones to assign");
    }
}
