use rsbwapi::*;

trait CanBuild {
    fn can_build_at(&self, loc: &TilePosition) -> bool;
    fn bounds(&self) -> (TilePosition, TilePosition);
    fn debug_rect(&self, tl: ScaledPosition<1>, br: ScaledPosition<1>, color: Color);
}

struct GameCanBuild<'a> {
    game: &'a Game,
    builder: &'a Unit,
    building_type: UnitType,
}

impl<'a> CanBuild for GameCanBuild<'a> {
    fn can_build_at(&self, loc: &TilePosition) -> bool {
        self.game
            .can_build_here(self.builder, *loc, self.building_type, true)
            .unwrap_or(false)
    }

    fn bounds(&self) -> (TilePosition, TilePosition) {
        (
            TilePosition { x: 0, y: 0 },
            TilePosition {
                x: self.game.map_width(),
                y: self.game.map_height(),
            },
        )
    }

    fn debug_rect(&self, tl: ScaledPosition<1>, br: ScaledPosition<1>, color: Color) {
        self.game.draw_box_map(tl, br, color, false);
    }
}

pub fn position_building(
    game: &Game,
    bt: UnitType,
    builder: &Unit,
    gas_locs: Vec<&TilePosition>,
) -> Option<TilePosition> {
    let checker = GameCanBuild {
        game,
        builder,
        building_type: bt,
    };
    match bt {
        UnitType::Zerg_Hatchery => position_new_base(game, builder, gas_locs),
        _ if bt.is_building() => position_near_hatch(game, &checker),
        _ => None,
    }
}

fn position_new_base(
    game: &Game,
    builder: &Unit,
    gas_locs: Vec<&TilePosition>,
) -> Option<TilePosition> {
    let hatches = get_hatches(game);
    let bt = UnitType::Zerg_Hatchery;
    // sort geysers by how far they are from our hatcheries
    let mut gas_locs: Vec<_> = gas_locs
        .into_iter()
        .map(|tp| {
            (
                hatches
                    .iter()
                    .map(|h| tp.chebyshev_distance(h.get_tile_position()))
                    .min()
                    .unwrap_or(999),
                tp,
            )
        })
        .collect();
    gas_locs.sort_by_key(|(d, _tp)| *d);
    let checker = GameCanBuild {
        game,
        builder,
        building_type: bt,
    };
    // first hatch is placed dist=5 from its geyser, so look for a geyser that
    // is further than that to indicate it hasn't been expanded to yet
    for (dist, g) in gas_locs {
        if dist <= 6 {
            continue;
        }
        let base_near_geyser = position_near(&checker, g.clone(), true);
        if base_near_geyser.is_some() {
            return base_near_geyser;
        }
    }
    position_near_hatch(game, &checker)
}

fn get_hatches(game: &Game) -> Vec<Unit> {
    if let Some(self_) = game.self_() {
        self_
            .get_units()
            .into_iter()
            .filter(|u| {
                // intention: hatches, lairs and hives but _not_ incomplete hatches
                u.get_type().is_successor_of(UnitType::Zerg_Hatchery)
                    && u.get_build_type() != UnitType::Zerg_Hatchery
            })
            .collect()
    } else {
        vec![]
    }
}

fn position_near_hatch(game: &Game, checker: &dyn CanBuild) -> Option<TilePosition> {
    for hatch in get_hatches(game) {
        let hatch_pos = hatch.get_tile_position();
        // println!("looking near hatch at {}", hatch.get_tile_position());
        let near_hatch = position_near(checker, hatch_pos, false);
        if near_hatch.is_some() {
            return near_hatch;
        }
    }
    None
}

fn position_near(
    checker: &dyn CanBuild,
    location: TilePosition,
    closest: bool,
) -> Option<TilePosition> {
    let search_radius = 10; // hatch width=4, pool width=3
    position_near_radius(checker, location, search_radius, search_radius, closest)
}

fn position_near_radius(
    checker: &dyn CanBuild,
    location: TilePosition,
    search_width: i32,
    search_height: i32,
    closest: bool,
) -> Option<TilePosition> {
    let TilePosition { x, y } = location;
    let (top_left, bottom_right) = checker.bounds();

    // search in a grid centered on the initial location
    use std::cmp::{max, min};
    let tl_x = max(x - search_width, top_left.x);
    let tl_y = max(y - search_width, top_left.y);
    let br_x = min(x + search_height, bottom_right.x);
    let br_y = min(y + search_height, bottom_right.y);

    let tl = TilePosition { x: tl_x, y: tl_y }.to_position();
    let br = TilePosition { x: br_x, y: br_y }.to_position();
    checker.debug_rect(tl, br, Color::Red);

    let mut matches = building_pos_search(tl_x, br_x, tl_y, br_y, checker);
    if closest {
        matches.sort_by_cached_key(|bl| (bl.distance(location) * 1000.0) as i32);
    }
    return matches.into_iter().filter(|m| *m != location).next();
}

#[allow(dead_code)]
fn position_anywhere(checker: &dyn CanBuild) -> Option<TilePosition> {
    let (top_left, bottom_right) = checker.bounds();
    building_pos_search(
        top_left.x,
        bottom_right.x,
        top_left.y,
        bottom_right.y,
        checker,
    )
    .into_iter()
    .next()
}

fn building_pos_search(
    x_start: i32,
    x_end: i32,
    y_start: i32,
    y_end: i32,
    checker: &dyn CanBuild,
) -> Vec<TilePosition> {
    let mut matches = vec![];
    for y in y_start..y_end {
        for x in x_start..x_end {
            let loc = TilePosition { x, y };
            if checker.can_build_at(&loc) {
                matches.push(loc);
            }
        }
    }
    matches
}

#[cfg(test)]
mod test {
    use rsbwapi::TilePosition;

    use super::{building_pos_search, position_near, CanBuild};

    struct FakeChecker {
        allowed: Vec<TilePosition>,
    }
    impl CanBuild for FakeChecker {
        fn can_build_at(&self, loc: &TilePosition) -> bool {
            return self.allowed.iter().find(|l| *l == loc).is_some();
        }

        fn bounds(&self) -> (TilePosition, TilePosition) {
            (TilePosition { x: 0, y: 0 }, TilePosition { x: 100, y: 100 })
        }
        fn debug_rect(
            &self,
            tl: rsbwapi::ScaledPosition<1>,
            br: rsbwapi::ScaledPosition<1>,
            _color: rsbwapi::Color,
        ) {
            println!("tl={:?}, br={:?}", tl, br);
        }
    }

    #[test]
    fn test_build_pos_search() {
        let checker = FakeChecker {
            allowed: vec![TilePosition { x: 9, y: 9 }],
        };
        assert_eq!(
            building_pos_search(0, 100, 0, 100, &checker),
            vec![TilePosition { x: 9, y: 9 }],
            "normal search failed"
        );
        assert_eq!(
            building_pos_search(0, 5, 0, 100, &checker),
            vec![],
            "restricted bounds search failed"
        );
    }

    #[test]
    fn test_build_pos_search_nowhere() {
        let checker = FakeChecker { allowed: vec![] };
        assert_eq!(
            building_pos_search(0, 100, 0, 100, &checker),
            vec![],
            "normal search failed"
        );
        assert_eq!(
            building_pos_search(100, 999, -1390, -30, &checker),
            vec![],
            "search out of bounds failed"
        );
        assert_eq!(
            building_pos_search(100, 0, 100, 0, &checker),
            vec![],
            "search backwards failed"
        );
    }

    #[test]
    fn test_find_near_loc() {
        let wanted = TilePosition { x: 49, y: 49 };
        let other = TilePosition { x: 44, y: 48 };
        let near_loc = TilePosition { x: 50, y: 50 };

        let checker = FakeChecker {
            allowed: vec![other.clone(), wanted.clone(), TilePosition { x: 80, y: 80 }],
        };
        assert_eq!(
            position_near(&checker, near_loc, true),
            Some(wanted),
            "find closest failed"
        );
        assert_eq!(
            position_near(&checker, near_loc, false),
            Some(other),
            "find near failed"
        );

        let no_match = TilePosition { x: 100, y: 100 };
        assert_eq!(
            position_near(&checker, no_match, true),
            None,
            "find near unexpected match"
        );
    }

    #[test]
    fn test_find_near_dont_return_given_loc() {
        let near_loc = TilePosition { x: 50, y: 50 };
        let checker = FakeChecker {
            allowed: vec![near_loc],
        };
        assert_eq!(
            position_near(&checker, near_loc, true),
            None,
            "returned the given query loc"
        );
    }
}
