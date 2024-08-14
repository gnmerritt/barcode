use rsbwapi::*;

trait CanBuild {
    fn can_build_at(&self, loc: &TilePosition) -> bool;
    fn bounds(&self) -> (TilePosition, TilePosition);
    fn width(&self) -> i32;
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
            .can_build_here(self.builder, (loc.x, loc.y), self.building_type, true)
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

    fn debug_rect(&self, tl: ScaledPosition<1>, br: ScaledPosition<1>, color: Color) {
        self.game.draw_box_map(tl, br, color, false);
    }
}

pub fn position_building(game: &Game, bt: UnitType, builder: &Unit) -> Option<TilePosition> {
    let checker = GameCanBuild {
        game,
        builder,
        building_type: bt,
    };
    match bt {
        UnitType::Zerg_Hatchery => position_new_base(game, builder),
        _ if bt.is_building() => position_near_hatch(game, &checker),
        _ => None,
    }
}

fn position_new_base(game: &Game, builder: &Unit) -> Option<TilePosition> {
    let hatches: Vec<_> = game
        .self_()
        .unwrap()
        .get_units()
        .into_iter()
        .filter(|u| u.get_type() == UnitType::Zerg_Hatchery)
        .collect();
    let bt = UnitType::Zerg_Hatchery;
    let mut geysers = game.get_geysers();
    // sort geysers by how far they are from our hatcheries
    geysers.sort_by_cached_key(|g| hatches.iter().map(|h| g.get_distance(h)).min());
    let checker = GameCanBuild {
        game,
        builder,
        building_type: bt,
    };
    println!("PNB post-sort geysers={:?}", &geysers);
    // assume for now that each hatch is next to a unique geyser, so those
    // will be the closest and not where we should build the next hatch
    //
    // this doesn't work at the moment, also geysers go away when we build an extractor on them
    for g in geysers.iter().skip(hatches.len()) {
        let base_near_geyser = position_near(&checker, g.get_tile_position());
        if base_near_geyser.is_some() {
            return base_near_geyser;
        }
    }
    let near_hatch = position_near_hatch(game, &checker);
    if near_hatch.is_some() {
        return near_hatch;
    }
    println!("positing anywhere, near hatch failed");
    position_anywhere(&checker)
}

fn position_near_hatch(game: &Game, checker: &dyn CanBuild) -> Option<TilePosition> {
    println!("positioning near existing hatches");
    let hatches: Vec<_> = game
        .self_()
        .unwrap()
        .get_units()
        .into_iter()
        .filter(|u| u.get_type() == UnitType::Zerg_Hatchery)
        .collect();
    for hatch in hatches {
        let hatch_pos = hatch.get_tile_position();
        println!(
            "looking hatch at {}, checker width={}",
            hatch.get_tile_position(),
            checker.width()
        );
        let near_hatch = position_near(checker, hatch_pos);
        if near_hatch.is_some() {
            return near_hatch;
        }
    }
    None
}

fn position_near(checker: &dyn CanBuild, location: TilePosition) -> Option<TilePosition> {
    let TilePosition { x, y } = location;
    let (top_left, bottom_right) = checker.bounds();

    // search in a grid centered on the initial location
    use std::cmp::{max, min};
    let search_radius = 3 * checker.width();
    let tl_x = max(x - search_radius, top_left.x);
    let tl_y = max(y - search_radius, top_left.y);
    let br_x = min(x + search_radius, bottom_right.x);
    let br_y = min(y + search_radius, bottom_right.y);

    let tl = TilePosition { x: tl_x, y: tl_y }.to_position();
    let br = TilePosition { x: br_x, y: br_y }.to_position();
    checker.debug_rect(tl, br, Color::Red);

    let mut matches = building_pos_search(tl_x, br_x, tl_y, br_y, checker);
    matches.sort_by_cached_key(|bl| bl.chebyshev_distance(location));
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
        fn width(&self) -> i32 {
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
        let checker = FakeChecker {
            allowed: vec![wanted.clone(), TilePosition { x: 80, y: 80 }],
        };
        let near_loc = TilePosition { x: 50, y: 50 };
        assert_eq!(
            position_near(&checker, near_loc),
            Some(wanted),
            "find near failed"
        );

        let no_match = TilePosition { x: 100, y: 100 };
        assert_eq!(
            position_near(&checker, no_match),
            None,
            "find near unexpected match"
        );
    }
}
