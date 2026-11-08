//! Ear-clipping triangulation for polygon fill (outer ring only, no holes).
//! Pure, offline, panic-free: every index goes through `.get()`, the loop
//! has a quadratic guard, degenerate input yields partial/empty output
//! instead of hanging. Canvas and headless preview share this so pixels agree.

/// Twice the signed area of (a, b, c). Positive when c is left of a->b.
fn area2(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Strict-interior point-in-triangle (edges excluded so shared edges never
/// block an ear). Barycentric with epsilon guards.
fn point_in_tri_strict(p: (f64, f64), a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> bool {
    let d = (b.1 - c.1) * (a.0 - c.0) + (c.0 - b.0) * (a.1 - c.1);
    if d.abs() < 1e-12 {
        return false;
    }
    let l1 = ((b.1 - c.1) * (p.0 - c.0) + (c.0 - b.0) * (p.1 - c.1)) / d;
    let l2 = ((c.1 - a.1) * (p.0 - c.0) + (a.0 - c.0) * (p.1 - c.1)) / d;
    let l3 = 1.0 - l1 - l2;
    let e = 1e-9;
    l1 > e && l2 > e && l3 > e
}

/// Triangulate a simple polygon ring. Returns flat index triples into
/// `points` (dedupe the closing vertex first; indices refer to the
/// deduped order only if the input had no consecutive duplicates... no:
/// indices refer to the ORIGINAL slice, closing duplicates skipped).
/// Holes are not supported yet (returned tris cover the outer ring).
pub fn earcut_tris(points: &[(f64, f64)]) -> Vec<usize> {
    // Map to unique-vertex indices; remember original positions.
    let mut uniq: Vec<usize> = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let dup = match uniq.last() {
            Some(&j) => points.get(j).is_some_and(|q| q == p),
            None => false,
        };
        if !dup {
            uniq.push(i);
        }
    }
    // Drop a closing duplicate.
    if uniq.len() >= 2 {
        if let (Some(&first), Some(&last)) = (uniq.first(), uniq.last()) {
            if let (Some(a), Some(b)) = (points.get(first), points.get(last)) {
                if a == b {
                    uniq.pop();
                }
            }
        }
    }
    if uniq.len() < 3 {
        return Vec::new();
    }
    let at =
        |idx: usize| -> Option<(f64, f64)> { uniq.get(idx).and_then(|&i| points.get(i)).copied() };
    // Signed area over the working ring to fix orientation (CCW wins).
    let mut area = 0.0;
    for i in 0..uniq.len() {
        if let (Some(a), Some(b)) = (at(i), at((i + 1) % uniq.len())) {
            area += a.0 * b.1 - b.0 * a.1;
        }
    }
    let mut ring: Vec<usize> = (0..uniq.len()).collect();
    if area < 0.0 {
        ring.reverse();
    }
    let mut tris: Vec<usize> = Vec::new();
    let mut guard = ring.len() * ring.len() + 1;
    while ring.len() > 3 && guard > 0 {
        guard -= 1;
        let n = ring.len();
        let mut clipped = false;
        for i in 0..n {
            let (ia, ib, ic) = match (
                ring.get((i + n - 1) % n).copied(),
                ring.get(i).copied(),
                ring.get((i + 1) % n).copied(),
            ) {
                (Some(a), Some(b), Some(c)) => (a, b, c),
                _ => continue,
            };
            let (a, b, c) = match (at(ia), at(ib), at(ic)) {
                (Some(a), Some(b), Some(c)) => (a, b, c),
                _ => continue,
            };
            if area2(a, b, c) <= 1e-12 {
                continue; // reflex or collinear: not an ear tip
            }
            let mut blocked = false;
            for (j, &idx) in ring.iter().enumerate() {
                if j == i || j == (i + n - 1) % n || j == (i + 1) % n {
                    continue;
                }
                if let Some(p) = at(idx) {
                    if point_in_tri_strict(p, a, b, c) {
                        blocked = true;
                        break;
                    }
                }
            }
            if blocked {
                continue;
            }
            tris.extend_from_slice(&[ia, ib, ic]);
            if i < ring.len() {
                ring.remove(i);
            }
            clipped = true;
            break;
        }
        if !clipped {
            break; // degenerate remainder: return the partial set
        }
    }
    if ring.len() == 3 {
        tris.extend_from_slice(&[ring[0], ring[1], ring[2]]);
    }
    // Map back through uniq to ORIGINAL input indices.
    tris.iter().filter_map(|&u| uniq.get(u).copied()).collect()
}

/// Polygon area via shoelace (for tests: triangulated area must match).
pub fn ring_area(ring: &[(f64, f64)]) -> f64 {
    let mut area = 0.0;
    for i in 0..ring.len() {
        if let (Some(&a), Some(&b)) = (ring.get(i), ring.get((i + 1) % ring.len())) {
            area += a.0 * b.1 - b.0 * a.1;
        }
    }
    area.abs() / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_is_two_triangles() {
        let sq = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert_eq!(earcut_tris(&sq).len(), 6);
    }

    #[test]
    fn triangle_is_itself() {
        let tri = vec![(0.0, 0.0), (2.0, 0.0), (0.0, 2.0)];
        assert_eq!(earcut_tris(&tri).len(), 3);
    }

    #[test]
    fn concave_area_preserved() {
        // C-shape: triangulation must preserve area (no hull fill).
        let c = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 1.0),
            (1.0, 1.0),
            (1.0, 3.0),
            (4.0, 3.0),
            (4.0, 4.0),
            (0.0, 4.0),
        ];
        let tris = earcut_tris(&c);
        assert!(!tris.is_empty());
        let mut t_area = 0.0;
        for t in tris.as_chunks::<3>().0 {
            if let (Some(&ia), Some(&ib), Some(&ic)) = (t.first(), t.get(1), t.get(2)) {
                if let (Some(&a), Some(&b), Some(&d)) = (c.get(ia), c.get(ib), c.get(ic)) {
                    t_area += area2(a, b, d).abs() / 2.0;
                }
            }
        }
        let poly = ring_area(&c);
        assert!((t_area - poly).abs() < 1e-6, "tri {t_area} vs poly {poly}");
    }

    #[test]
    fn degenerate_input_yields_empty() {
        assert!(earcut_tris(&[]).is_empty());
        assert!(earcut_tris(&[(0.0, 0.0)]).is_empty());
        assert!(earcut_tris(&[(0.0, 0.0), (1.0, 1.0)]).is_empty());
        // All identical: dedupes to one point.
        assert!(earcut_tris(&[(1.0, 1.0), (1.0, 1.0), (1.0, 1.0)]).is_empty());
    }

    #[test]
    fn closed_ring_matches_open_ring() {
        let open = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let mut closed = open.clone();
        closed.push((0.0, 0.0));
        assert_eq!(earcut_tris(&open), earcut_tris(&closed));
    }
}
