use rsbwapi::{Game, TilePosition, Unit, UnitId, UnitType};
use std::collections::VecDeque;

pub(crate) struct Scout {
    unit: Unit,
    current_destination: Option<TilePosition>,
    destinations: VecDeque<TilePosition>,
}

impl Scout {
    pub fn new(unit: Unit) -> Self {
        Scout {
            unit,
            current_destination: None,
            destinations: VecDeque::new(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.unit.exists() && self.unit.get_type() == UnitType::Zerg_Drone
    }

    pub fn is_done(&self) -> bool {
        self.destinations.is_empty() && self.current_destination.is_none() && self.unit.is_idle()
    }

    pub fn get_id(&self) -> UnitId {
        self.unit.get_id()
    }

    pub fn on_frame(&mut self, game: &Game) {
        match self.current_destination {
            Some(dest)
                if self
                    .unit
                    .get_position()
                    .chebyshev_distance(dest.to_position())
                    < 3
                    || (self.unit.is_stuck() && !self.unit.is_moving())
                    || game.is_visible(dest) =>
            {
                println!("scout {} arrived at {}", self.unit.get_id(), dest);
                self.current_destination = None;
            }
            Some(dest) => {
                self.unit.move_(dest.to_position()).ok();
            }
            None => self.current_destination = self.destinations.pop_front(),
        }
    }

    pub fn go_later(&mut self, dest: TilePosition) {
        self.destinations.push_back(dest);
    }

    #[allow(unused)]
    pub fn go_now(&mut self, dest: TilePosition) {
        println!("sending scout {} to {} NOW", self.unit.get_id(), dest);
        if let Some(current) = self.current_destination {
            self.destinations.push_front(current);
            self.current_destination = None;
        }
        self.destinations.push_front(dest);
    }
}
