use crate::seen::HaveSeen;
use rsbwapi::*;

trait CanBuild {
    fn can_build_at(&self, loc: TilePosition) -> bool;
    fn bounds(&self) -> (TilePosition, TilePosition);
    #[allow(unused)]
    fn width(&self) -> i32;
    #[allow(unused)]
    fn height(&self) -> i32;
    fn debug_rect(&self, tl: ScaledPosition<1>, br: ScaledPosition<1>, color: Color);
}

struct GameCanBuild<'a> {
    game: &'a Game,
    builder: &'a Unit,
    building_type: UnitType,
}

impl<'a> CanBuild for GameCanBuild<'a> {
    fn can_build_at(&self, loc: TilePosition) -> bool {
        self.game
            .can_build_here(self.builder, loc, self.building_type, false)
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

    fn width(&self) -> i32 {
        self.building_type.tile_width()
    }

    fn height(&self) -> i32 {
        self.building_type.tile_height()
    }

    fn debug_rect(&self, tl: ScaledPosition<1>, br: ScaledPosition<1>, color: Color) {
        self.game.draw_box_map(tl, br, color, false);
    }
}

pub fn position_building(
    game: &Game,
    bt: UnitType,
    builder: &Unit,
    seen: &HaveSeen,
) -> Option<TilePosition> {
    let checker = GameCanBuild {
        game,
        builder,
        building_type: bt,
    };
    match bt {
        UnitType::Zerg_Hatchery => position_new_base(game, builder, seen),
        _ if bt.is_building() => position_near_hatch(game, &checker),
        _ => None,
    }
}

fn position_new_base(game: &Game, builder: &Unit, seen: &HaveSeen) -> Option<TilePosition> {
    let hatches = get_hatches(game);
    let bt = UnitType::Zerg_Hatchery;
    // sort geysers by how far they are from our hatcheries
    let mut gas_locs: Vec<_> = seen
        .get_gas_locs()
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

    let mineral_locs = seen.get_mineral_locs();
    // first hatch is placed dist=5 from its geyser, so look for a geyser that
    // is further than that to indicate it hasn't been expanded to yet
    for (dist, gas) in gas_locs {
        if dist <= 7 {
            continue;
        }
        let mut mins_near_gas: Vec<TilePosition> = mineral_locs
            .iter()
            .filter(|m| m.chebyshev_distance(*gas) < 12)
            .map(|p| (*p).clone())
            .collect();
        mins_near_gas.push(gas.clone());
        let center_mins_gas = cartesian_center(&mins_near_gas).expect("gas locs always present");

        // be near the gas & also the average position of the mins
        let locs = vec![gas.clone(), center_mins_gas];
        let center_locs = cartesian_center(&locs).expect("has gas and cmg");
        checker.debug_rect(
            center_locs.to_position(),
            (center_locs + TilePosition { x: 1, y: 1 }).to_position(),
            Color::Orange,
        );
        let base_near_geyser = position_near_radius(
            &checker,
            &center_locs,
            &vec![&gas, &center_mins_gas],
            7,
            7,
            true,
        );
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
        let near_hatch = position_near(checker, &hatch_pos, false);
        if near_hatch.is_some() {
            return near_hatch;
        }
    }
    None
}

// TODO clean this mess of dispatches up once it's all working. search options object?
fn position_near(
    checker: &dyn CanBuild,
    location: &TilePosition,
    closest: bool,
) -> Option<TilePosition> {
    let search_radius = 10; // hatch width=4, pool width=3
    position_near_radius(
        checker,
        location,
        &vec![location],
        search_radius,
        search_radius,
        closest,
    )
}

fn position_near_radius(
    checker: &dyn CanBuild,
    center: &TilePosition,
    locations: &Vec<&TilePosition>,
    search_width: i32,
    search_height: i32,
    closest: bool,
) -> Option<TilePosition> {
    let TilePosition { x, y } = center;
    let (top_left, bottom_right) = checker.bounds();

    // search in a grid centered on the initial location
    use std::cmp::{max, min};
    let tl_x = max(x - search_width, top_left.x);
    let tl_y = max(y - search_height, top_left.y);
    let br_x = min(x + search_width, bottom_right.x);
    let br_y = min(y + search_height, bottom_right.y);

    let tl = TilePosition { x: tl_x, y: tl_y }.to_position();
    let br = TilePosition { x: br_x, y: br_y }.to_position();
    checker.debug_rect(tl, br, Color::Red);

    let mut matches = building_pos_search(tl_x, br_x, tl_y, br_y, checker);
    if closest {
        matches.sort_by_cached_key(|tl| {
            locations
                .iter()
                .map(|l| l.distance_squared(*tl))
                .sum::<u32>()
        });
    }
    /*
    for m in matches.iter() {
        checker.debug_rect(
            m.to_position(),
            (*m + TilePosition {
                x: checker.width(),
                y: checker.height(),
            })
            .to_position(),
            Color::Cyan,
        );
    }
    */
    return matches.into_iter().next();
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
    for y in y_start..y_end + 1 {
        for x in x_start..x_end + 1 {
            let loc = TilePosition { x, y };
            if checker.can_build_at(loc) {
                matches.push(loc);
            }
        }
    }
    matches
}

pub fn cartesian_center(points: &Vec<TilePosition>) -> Option<TilePosition> {
    let len = points.len();
    if len == 0 {
        return None;
    }
    let x_mean = points.iter().map(|p| p.x).sum::<i32>() / len as i32;
    let y_mean = points.iter().map(|p| p.y).sum::<i32>() / len as i32;
    Some(TilePosition {
        x: x_mean,
        y: y_mean,
    })
}

pub(crate) fn tile_position_towards(
    from: &TilePosition,
    distance: i32,
    towards: &TilePosition,
) -> TilePosition {
    let x_dir = towards.x - from.x;
    let y_dir = towards.y - from.y;

    let mut x_delta = distance;
    if x_dir < 0 {
        x_delta *= -1;
    }
    let mut y_delta = distance;
    if y_dir < 0 {
        y_delta *= -1;
    }

    *from
        + TilePosition {
            x: x_delta,
            y: y_delta,
        }
}

#[cfg(test)]
mod test {
    use super::{
        building_pos_search, cartesian_center, position_near, tile_position_towards, CanBuild,
    };
    use rsbwapi::TilePosition;

    struct FakeChecker {
        allowed: Vec<TilePosition>,
    }
    impl CanBuild for FakeChecker {
        fn can_build_at(&self, loc: TilePosition) -> bool {
            return self.allowed.iter().find(|l| **l == loc).is_some();
        }

        fn bounds(&self) -> (TilePosition, TilePosition) {
            (TilePosition { x: 0, y: 0 }, TilePosition { x: 100, y: 100 })
        }
        fn width(&self) -> i32 {
            1
        }
        fn height(&self) -> i32 {
            1
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
            position_near(&checker, &near_loc, true),
            Some(wanted),
            "find closest failed"
        );
        assert_eq!(
            position_near(&checker, &near_loc, false),
            Some(other),
            "find near failed"
        );

        let no_match = TilePosition { x: 100, y: 100 };
        assert_eq!(
            position_near(&checker, &no_match, true),
            None,
            "find near unexpected match"
        );
    }

    #[test]
    fn test_center() {
        assert_eq!(cartesian_center(&vec![]), None, "empty list of points");

        let points = vec![TilePosition { x: 0, y: 10 }, TilePosition { x: 2, y: 16 }];
        assert_eq!(
            cartesian_center(&points),
            Some(TilePosition { x: 1, y: 13 })
        );
    }

    #[test]
    fn test_tile_position_towards() {
        assert_eq!(
            tile_position_towards(
                &TilePosition { x: 0, y: 0 },
                5,
                &TilePosition { x: 10, y: 10 }
            ),
            TilePosition { x: 5, y: 5 },
            "easy up and to the right"
        );
        assert_eq!(
            tile_position_towards(
                &TilePosition { x: 0, y: 0 },
                0,
                &TilePosition { x: 10, y: 10 }
            ),
            TilePosition { x: 0, y: 0 },
            "going nowhere"
        );
        assert_eq!(
            tile_position_towards(
                &TilePosition { x: 0, y: 0 },
                5,
                &TilePosition { x: -10, y: 10 }
            ),
            TilePosition { x: -5, y: 5 },
            "up and left"
        );
    }
}
