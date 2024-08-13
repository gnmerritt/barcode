use rsbwapi::*;

#[derive(PartialEq, Copy, Clone, Debug)]
pub(crate) struct BuildLoc {
    pub x: i32,
    pub y: i32,
}

trait CanBuild {
    fn can_build_at(&self, loc: &BuildLoc) -> bool;
    fn bounds(&self) -> (BuildLoc, BuildLoc);
    fn width(&self) -> i32;
}

struct GameCanBuild<'a> {
    game: &'a Game,
    builder: &'a Unit,
    building_type: UnitType,
}

impl<'a> CanBuild for GameCanBuild<'a> {
    fn can_build_at(&self, loc: &BuildLoc) -> bool {
        self.game
            .can_build_here(self.builder, (loc.x, loc.y), self.building_type, true)
            .unwrap_or(false)
    }

    fn bounds(&self) -> (BuildLoc, BuildLoc) {
        (
            BuildLoc { x: 0, y: 0 },
            BuildLoc {
                x: self.game.map_width(),
                y: self.game.map_height(),
            },
        )
    }

    fn width(&self) -> i32 {
        self.building_type.tile_width()
    }
}

pub fn position_building(game: &Game, bt: UnitType, builder: &Unit) -> Option<BuildLoc> {
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

fn position_new_base(game: &Game, builder: &Unit) -> Option<BuildLoc> {
    let hatches: Vec<_> = game
        .self_()
        .unwrap()
        .get_units()
        .into_iter()
        .filter(|u| u.get_type() == UnitType::Zerg_Hatchery)
        .collect();
    // there's a different set of build checks for new bases, use a special building type
    let bt = UnitType::Special_Start_Location;
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

fn position_near_hatch(game: &Game, checker: &dyn CanBuild) -> Option<BuildLoc> {
    println!("positioning near existing hatches");
    let hatches: Vec<_> = game
        .self_()
        .unwrap()
        .get_units()
        .into_iter()
        .filter(|u| u.get_type() == UnitType::Zerg_Hatchery)
        .collect();
    for hatch in hatches {
        let near_hatch = position_near(checker, hatch.get_tile_position());
        if near_hatch.is_some() {
            return near_hatch;
        }
    }
    None
}

fn position_near(checker: &dyn CanBuild, location: TilePosition) -> Option<BuildLoc> {
    let TilePosition { x, y } = location;
    let (top_left, bottom_right) = checker.bounds();

    // search in a grid centered on the initial location
    use std::cmp::{max, min};
    let search_radius = 4 * checker.width();
    let tl_x = max(x - search_radius, top_left.x);
    let tl_y = max(y - search_radius, top_left.y);
    let br_x = min(x + search_radius, bottom_right.x);
    let br_y = min(y + search_radius, bottom_right.y);

    building_pos_search(tl_x, br_x, tl_y, br_y, checker)
}

#[allow(dead_code)]
fn position_anywhere(checker: &dyn CanBuild) -> Option<BuildLoc> {
    let (top_left, bottom_right) = checker.bounds();
    building_pos_search(
        top_left.x,
        bottom_right.x,
        top_left.y,
        bottom_right.y,
        checker,
    )
}

fn building_pos_search(
    x_start: i32,
    x_end: i32,
    y_start: i32,
    y_end: i32,
    checker: &dyn CanBuild,
) -> Option<BuildLoc> {
    for y in y_start..y_end {
        for x in x_start..x_end {
            let loc = BuildLoc { x, y };
            if checker.can_build_at(&loc) {
                return Some(loc);
            }
        }
    }
    None
}

#[cfg(test)]
mod test {
    use rsbwapi::TilePosition;

    use super::{building_pos_search, position_near, BuildLoc, CanBuild};

    struct FakeChecker {
        allowed: Vec<BuildLoc>,
    }
    impl CanBuild for FakeChecker {
        fn can_build_at(&self, loc: &BuildLoc) -> bool {
            return self.allowed.iter().find(|l| *l == loc).is_some();
        }

        fn bounds(&self) -> (BuildLoc, BuildLoc) {
            (BuildLoc { x: 0, y: 0 }, BuildLoc { x: 100, y: 100 })
        }
        fn width(&self) -> i32 {
            1
        }
    }

    #[test]
    fn test_build_pos_search() {
        let checker = FakeChecker {
            allowed: vec![BuildLoc { x: 9, y: 9 }],
        };
        assert_eq!(
            building_pos_search(0, 100, 0, 100, &checker),
            Some(BuildLoc { x: 9, y: 9 }),
            "normal search failed"
        );
        assert_eq!(
            building_pos_search(0, 5, 0, 100, &checker),
            None,
            "restricted bounds search failed"
        );
    }

    #[test]
    fn test_build_pos_search_nowhere() {
        let checker = FakeChecker { allowed: vec![] };
        assert_eq!(
            building_pos_search(0, 100, 0, 100, &checker),
            None,
            "normal search failed"
        );
        assert_eq!(
            building_pos_search(100, 999, -1390, -30, &checker),
            None,
            "search out of bounds failed"
        );
        assert_eq!(
            building_pos_search(100, 0, 100, 0, &checker),
            None,
            "search backwards failed"
        );
    }

    #[test]
    fn test_find_near_loc() {
        let wanted = BuildLoc { x: 49, y: 49 };
        let checker = FakeChecker {
            allowed: vec![wanted.clone(), BuildLoc { x: 80, y: 80 }],
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
