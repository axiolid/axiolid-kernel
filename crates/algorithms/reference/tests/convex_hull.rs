use axiolid_core::Point2;
use axiolid_reference::{minimum_area_rectangle, strict_convex_hull};

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

#[test]
fn hull_is_stable_and_strict() {
    let points = [
        p(0., 0.),
        p(2., 0.),
        p(2., 2.),
        p(0., 2.),
        p(1., 0.),
        p(0., 0.),
    ];
    assert_eq!(strict_convex_hull(&points).unwrap(), vec![0, 1, 2, 3]);
}

#[test]
fn rectangle_has_expected_dimensions() {
    let points = [p(0., 0.), p(3., 0.), p(3., 2.), p(0., 2.)];
    let rectangle = minimum_area_rectangle(&points).unwrap();
    assert_eq!(rectangle.area(), 6.0);
    assert_eq!(rectangle.side_lengths(), [3.0, 2.0]);
}

#[test]
fn non_finite_input_is_rejected() {
    assert!(strict_convex_hull(&[p(0., 0.), p(f64::NAN, 0.)]).is_err());
}

fn turn(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> i128 {
    i128::from(b.0 - a.0) * i128::from(c.1 - a.1) - i128::from(b.1 - a.1) * i128::from(c.0 - a.0)
}

/// Independent Jarvis march over exact integer coordinates. It deliberately
/// shares neither Axiolid's predicate implementation nor its monotone-chain
/// control flow.
fn integer_oracle(points: &[(i64, i64)]) -> Vec<usize> {
    let start = (0..points.len()).min_by_key(|&i| (points[i], i)).unwrap();
    let mut result = vec![start];
    loop {
        let current = *result.last().unwrap();
        let mut next = (0..points.len())
            .find(|&i| points[i] != points[current])
            .unwrap();
        for candidate in 0..points.len() {
            if candidate == current || points[candidate] == points[current] {
                continue;
            }
            let side = turn(points[current], points[next], points[candidate]);
            let farther = (points[candidate].0 - points[current].0).pow(2)
                + (points[candidate].1 - points[current].1).pow(2)
                > (points[next].0 - points[current].0).pow(2)
                    + (points[next].1 - points[current].1).pow(2);
            if side < 0 || (side == 0 && farther) {
                next = candidate;
            }
        }
        if next == start {
            break;
        }
        result.push(next);
    }
    result
}

#[test]
fn certified_hull_matches_independent_integer_oracle() {
    let integers = [
        (2, 0),
        (0, 2),
        (3, 3),
        (0, 0),
        (3, 0),
        (1, 1),
        (0, 0),
        (0, 3),
    ];
    let points: Vec<_> = integers
        .iter()
        .map(|&(x, y)| p(x as f64, y as f64))
        .collect();
    assert_eq!(
        strict_convex_hull(&points).unwrap(),
        integer_oracle(&integers)
    );
}
