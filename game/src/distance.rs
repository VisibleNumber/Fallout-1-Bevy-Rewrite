/// Return the hex distance between two Fallout map tile IDs.
///
/// Fallout's 200-column odd-q offset grid maps to axial coordinates with
/// `r = row - (column + (column & 1)) / 2`. The cube-coordinate distance is
/// the largest absolute delta on q, r, or q+r. This matches the parity-based
/// neighbor offsets used by `navigation::neighbors`.
pub fn hex_distance(a: i32, b: i32) -> i32 {
    const WIDTH: i64 = 200;

    fn axial(tile: i32) -> (i64, i64) {
        let tile = i64::from(tile);
        let column = tile.rem_euclid(WIDTH);
        let row = tile.div_euclid(WIDTH);
        let r = row - (column + (column & 1)) / 2;
        (column, r)
    }

    let (aq, ar) = axial(a);
    let (bq, br) = axial(b);
    let dq = aq - bq;
    let dr = ar - br;
    let ds = dq + dr;
    dq.abs().max(dr.abs()).max(ds.abs()) as i32
}

/// Return the tiles on a straight hex line from `a` to `b`, including both
/// endpoints. Invalid map tile IDs produce an empty line.
pub fn hex_line(a: i32, b: i32) -> Vec<i32> {
    const WIDTH: i32 = 200;
    const TILE_COUNT: i32 = 40_000;
    if !(0..TILE_COUNT).contains(&a) || !(0..TILE_COUNT).contains(&b) {
        return Vec::new();
    }

    fn axial(tile: i32) -> (i32, i32) {
        let column = tile % WIDTH;
        let row = tile / WIDTH;
        (column, row - (column + (column & 1)) / 2)
    }

    fn round_cube(q: f64, r: f64, s: f64) -> (i32, i32) {
        let mut rq = q.round();
        let mut rr = r.round();
        let mut rs = s.round();
        let q_diff = (rq - q).abs();
        let r_diff = (rr - r).abs();
        let s_diff = (rs - s).abs();
        if q_diff > r_diff && q_diff > s_diff {
            rq = -rr - rs;
        } else if r_diff > s_diff {
            rr = -rq - rs;
        } else {
            rs = -rq - rr;
        }
        let _ = rs;
        (rq as i32, rr as i32)
    }

    fn axial_error(tile: i32, q: f64, r: f64, s: f64) -> f64 {
        let column = tile % WIDTH;
        let row = tile / WIDTH;
        let tile_q = f64::from(column);
        let tile_r = f64::from(row - (column + (column & 1)) / 2);
        let tile_s = -tile_q - tile_r;
        (tile_q - q).powi(2) + (tile_r - r).powi(2) + (tile_s - s).powi(2)
    }

    fn bounded_neighbors(tile: i32) -> impl Iterator<Item = i32> {
        let column = tile % WIDTH;
        let row = tile / WIDTH;
        let offsets = if column % 2 == 0 {
            [-1, WIDTH - 1, WIDTH, WIDTH + 1, 1, -WIDTH]
        } else {
            [-WIDTH - 1, -1, WIDTH, 1, 1 - WIDTH, -WIDTH]
        };
        offsets.into_iter().filter_map(move |delta| {
            let next = tile + delta;
            if !(0..TILE_COUNT).contains(&next) {
                return None;
            }
            let next_column = next % WIDTH;
            let next_row = next / WIDTH;
            ((next_column - column).abs() <= 1 && (next_row - row).abs() <= 1).then_some(next)
        })
    }

    let steps = hex_distance(a, b);
    if steps == 0 {
        return vec![a];
    }
    let (aq, ar) = axial(a);
    let (bq, br) = axial(b);
    let (as_, bs) = (-aq - ar, -bq - br);
    let mut line = Vec::with_capacity(steps as usize + 1);
    for index in 0..=steps {
        if index == 0 {
            line.push(a);
            continue;
        }
        if index == steps {
            line.push(b);
            continue;
        }
        let t = f64::from(index) / f64::from(steps);
        let q = f64::from(aq) + f64::from(bq - aq) * t;
        let r = f64::from(ar) + f64::from(br - ar) * t;
        let s = f64::from(as_) + f64::from(bs - as_) * t;
        let (rounded_q, rounded_r) = round_cube(q, r, s);
        let tile_q = rounded_q;
        let tile_r = rounded_r + (rounded_q + (rounded_q & 1)) / 2;
        let rounded_tile = tile_r * WIDTH + tile_q;
        let previous = *line.last().expect("the start tile is present");
        let tile = if (0..TILE_COUNT).contains(&rounded_tile)
            && hex_distance(previous, rounded_tile) == 1
            && hex_distance(rounded_tile, b) == steps - index
        {
            rounded_tile
        } else {
            bounded_neighbors(previous)
                .filter(|candidate| hex_distance(*candidate, b) == steps - index)
                .min_by(|left, right| {
                    axial_error(*left, q, r, s).total_cmp(&axial_error(*right, q, r, s))
                })
                .unwrap_or(rounded_tile)
        };
        if !(0..TILE_COUNT).contains(&tile) {
            return Vec::new();
        }
        line.push(tile);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::hex_distance;
    use std::collections::VecDeque;

    const WIDTH: i32 = 200;
    const HEIGHT: i32 = 200;
    const TILE_COUNT: usize = (WIDTH * HEIGHT) as usize;

    // Same parity deltas and row-wrap guards as navigation::neighbors.
    fn neighbors(tile: i32) -> impl Iterator<Item = i32> {
        let col = tile % WIDTH;
        let row = tile / WIDTH;
        let offsets = if col % 2 == 0 {
            [-1, WIDTH - 1, WIDTH, WIDTH + 1, 1, -WIDTH]
        } else {
            [-WIDTH - 1, -1, WIDTH, 1, 1 - WIDTH, -WIDTH]
        };
        offsets.into_iter().filter_map(move |delta| {
            let next = tile + delta;
            if !(0..WIDTH * HEIGHT).contains(&next) {
                return None;
            }
            let next_col = next % WIDTH;
            let next_row = next / WIDTH;
            ((next_col - col).abs() <= 1 && (next_row - row).abs() <= 1).then_some(next)
        })
    }

    fn breadth_first_distances(start: i32) -> Vec<i32> {
        let mut distances = vec![-1; TILE_COUNT];
        let mut queue = VecDeque::new();
        distances[start as usize] = 0;
        queue.push_back(start);
        while let Some(tile) = queue.pop_front() {
            let next_distance = distances[tile as usize] + 1;
            for next in neighbors(tile) {
                let next_distance_slot = &mut distances[next as usize];
                if *next_distance_slot != -1 {
                    continue;
                }
                *next_distance_slot = next_distance;
                queue.push_back(next);
            }
        }
        distances
    }

    #[test]
    fn every_navigation_neighbor_has_distance_one_and_is_reciprocal() {
        for tile in 0..WIDTH * HEIGHT {
            for next in neighbors(tile) {
                assert_eq!(hex_distance(tile, next), 1, "neighbor {tile} -> {next}");
                assert!(
                    neighbors(next).any(|candidate| candidate == tile),
                    "missing reverse edge {next} -> {tile}"
                );
            }
        }
    }

    #[test]
    fn sampled_distances_match_full_open_map_breadth_first_search() {
        let sources = [
            0, 1, 199, 200, 201, 10_199, 19_899, 20_000, 20_100, 39_800, 39_999,
        ];
        let targets = [
            0, 1, 2, 198, 199, 200, 201, 399, 10_199, 19_899, 20_000, 20_100, 20_199, 39_800,
            39_999,
        ];
        for source in sources {
            let distances = breadth_first_distances(source);
            for target in targets {
                assert_eq!(
                    hex_distance(source, target),
                    distances[target as usize],
                    "distance {source} -> {target} across offset parities",
                );
            }
        }
    }

    #[test]
    fn distance_is_symmetric_and_zero_for_same_tile() {
        for (a, b) in [(0, 39_999), (19_899, 20_100), (200, 201), (10_199, 20_000)] {
            assert_eq!(hex_distance(a, a), 0);
            assert_eq!(hex_distance(a, b), hex_distance(b, a));
        }
    }

    #[test]
    fn straight_lines_include_endpoints_and_follow_adjacent_tiles() {
        for (start, goal) in [
            (0, 39_999),
            (1, 19_998),
            (10_199, 20_000),
            (19_899, 20_100),
            (39_800, 199),
            (20_000, 20_000),
        ] {
            let line = super::hex_line(start, goal);
            assert_eq!(line.first(), Some(&start));
            assert_eq!(line.last(), Some(&goal));
            assert_eq!(line.len(), hex_distance(start, goal) as usize + 1);
            assert!(line.iter().all(|tile| (0..WIDTH * HEIGHT).contains(tile)));
            assert!(
                line.windows(2)
                    .all(|pair| neighbors(pair[0]).any(|next| next == pair[1]))
            );
        }
        assert!(super::hex_line(-1, 10).is_empty());
        assert!(super::hex_line(10, 40_000).is_empty());
    }

    #[test]
    fn generic_non_tie_lines_reverse_exactly() {
        for (a, b) in [
            (10_000, 12_347),
            (2_000, 35_821),
            (8_123, 27_654),
            (14_222, 30_119),
        ] {
            let mut reverse = super::hex_line(b, a);
            reverse.reverse();
            assert_eq!(super::hex_line(a, b), reverse, "line {a} -> {b}");
        }
    }

    #[test]
    fn valid_map_lines_never_leave_tile_bounds() {
        for start in 0..WIDTH * HEIGHT {
            for goal in [0, WIDTH - 1, (HEIGHT - 1) * WIDTH, WIDTH * HEIGHT - 1] {
                let line = super::hex_line(start, goal);
                assert!(!line.is_empty(), "valid line {start} -> {goal}");
                assert_eq!(line.first(), Some(&start));
                assert_eq!(line.last(), Some(&goal));
                assert_eq!(line.len(), hex_distance(start, goal) as usize + 1);
                assert!(line.iter().all(|tile| (0..WIDTH * HEIGHT).contains(tile)));
                assert!(
                    line.windows(2)
                        .all(|pair| neighbors(pair[0]).any(|next| next == pair[1]))
                );
            }
        }
    }
}
