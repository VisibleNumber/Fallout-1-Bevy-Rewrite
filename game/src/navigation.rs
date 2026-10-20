use std::collections::VecDeque;

const WIDTH: i32 = 200;
const HEIGHT: i32 = 200;
const TILE_COUNT: usize = (WIDTH * HEIGHT) as usize;

/// Return the isometric center of a Fallout tile, with Bevy's y-up convention.
pub fn screen(tile: i32) -> (f32, f32) {
    let c = 199 - tile % WIDTH;
    let r = tile / WIDTH;
    let parity = c % 2;
    let x = 24 * c + 16 * r + 8 * parity + 16;
    let y = -(-6 * c + 12 * r + 6 * parity + 8);
    (x as f32, y as f32)
}

/// Return legal adjacent tiles in Fallout rotation order (0 through 5).
///
/// The parity-dependent offsets match `tile.cc`; coordinate checks prevent a
/// linear offset from wrapping across a map row or beyond the map boundary.
pub fn neighbors(tile: i32) -> Vec<(i32, usize)> {
    if !(0..(WIDTH * HEIGHT)).contains(&tile) {
        return Vec::new();
    }

    let col = tile % WIDTH;
    let row = tile / WIDTH;
    let offsets = if col % 2 == 0 {
        [-1, WIDTH - 1, WIDTH, WIDTH + 1, 1, -WIDTH]
    } else {
        [-WIDTH - 1, -1, WIDTH, 1, 1 - WIDTH, -WIDTH]
    };

    offsets
        .into_iter()
        .enumerate()
        .filter_map(|(direction, offset)| {
            let next = tile + offset;
            if !(0..(WIDTH * HEIGHT)).contains(&next) {
                return None;
            }
            let next_col = next % WIDTH;
            let next_row = next / WIDTH;
            if (next_col - col).abs() > 1 || (next_row - row).abs() > 1 {
                return None;
            }
            Some((next, direction))
        })
        .collect()
}

/// Find a shortest walkable path, excluding `start` and including `goal`.
/// Invalid tile IDs, a malformed walkability map, and unreachable goals return
/// an empty path.
pub fn route(start: i32, goal: i32, walkable: &[bool]) -> Vec<i32> {
    if walkable.len() != TILE_COUNT
        || !(0..(WIDTH * HEIGHT)).contains(&start)
        || !(0..(WIDTH * HEIGHT)).contains(&goal)
        || !walkable[start as usize]
        || !walkable[goal as usize]
    {
        return Vec::new();
    }
    if start == goal {
        return Vec::new();
    }

    let mut parent = vec![-2_i32; TILE_COUNT];
    let mut queue = VecDeque::new();
    parent[start as usize] = -1;
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        for (next, _) in neighbors(current) {
            let index = next as usize;
            if parent[index] != -2 || !walkable[index] {
                continue;
            }
            parent[index] = current;
            if next == goal {
                let mut path = vec![goal];
                let mut cursor = goal;
                while cursor != start {
                    cursor = parent[cursor as usize];
                    if cursor != start {
                        path.push(cursor);
                    }
                }
                path.reverse();
                return path;
            }
            queue.push_back(next);
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_map() -> Vec<bool> {
        vec![true; TILE_COUNT]
    }

    #[test]
    fn screen_centers_follow_isometric_formula_and_y_up() {
        assert_eq!(screen(0), (4800.0, 1180.0));
        assert_eq!(screen(199), (16.0, -8.0));
        assert_eq!(screen(200), (4816.0, 1168.0));
    }

    #[test]
    fn neighbors_are_reciprocal_and_keep_rotation_order() {
        for tile in [1000, 1001, 19_899, 20_000, 39_000] {
            for (next, direction) in neighbors(tile) {
                let opposite = (direction + 3) % 6;
                assert!(neighbors(next).contains(&(tile, opposite)));
            }
        }
        assert_eq!(neighbors(-1), Vec::<(i32, usize)>::new());
        assert_eq!(neighbors(40_000), Vec::<(i32, usize)>::new());
    }

    #[test]
    fn neighbors_clip_edges_without_linear_row_wrap() {
        for tile in [0, 199, 200, 39_800, 39_999] {
            for (next, _) in neighbors(tile) {
                assert!((0..40_000).contains(&next));
                assert!(((tile % WIDTH) - (next % WIDTH)).abs() <= 1);
                assert!(((tile / WIDTH) - (next / WIDTH)).abs() <= 1);
            }
        }
        assert_eq!(neighbors(0).len(), 3);
        assert_eq!(neighbors(39_999).len(), 3);
    }

    #[test]
    fn route_uses_shortest_walkable_detour_and_excludes_start() {
        let mut walkable = open_map();
        let start = 20_100;
        let mut pair = None;
        for (direct, _) in neighbors(start) {
            for (goal, _) in neighbors(direct) {
                if goal != start && !neighbors(start).iter().any(|(next, _)| *next == goal) {
                    let mut blocked = open_map();
                    blocked[direct as usize] = false;
                    if route(start, goal, &blocked).len() == 2 {
                        pair = Some((direct, goal));
                        break;
                    }
                }
            }
            if pair.is_some() {
                break;
            }
        }
        let (direct, goal) = pair.expect("open map has a two-step blocked detour");
        walkable[direct as usize] = false;

        let path = route(start, goal, &walkable);
        assert!(!path.is_empty());
        assert_ne!(path[0], start);
        assert_eq!(path.last(), Some(&goal));
        assert!(!path.contains(&direct));
        assert!(path.iter().all(|tile| walkable[*tile as usize]));
        let mut previous = start;
        for tile in &path {
            assert!(neighbors(previous).iter().any(|(next, _)| next == tile));
            previous = *tile;
        }
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn route_reports_unreachable_and_invalid_inputs_as_empty() {
        let mut walkable = vec![false; TILE_COUNT];
        walkable[1000] = true;
        walkable[1001] = true;
        assert!(route(1000, 2000, &walkable).is_empty());
        assert!(route(1000, 1001, &[]).is_empty());
        assert!(route(-1, 1001, &walkable).is_empty());
        assert!(route(1000, 40_000, &walkable).is_empty());
        assert_eq!(route(1000, 1001, &walkable), vec![1001]);
        walkable[1001] = false;
        assert!(route(1000, 1001, &walkable).is_empty());
        assert!(route(1000, 1000, &walkable).is_empty());
    }
}
