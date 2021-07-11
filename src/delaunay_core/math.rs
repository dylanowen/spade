use crate::{LineSideInfo, Point2, SpadeNum};
use num_traits::Float;

pub struct PointProjection<S> {
    factor: S,
    length_2: S,
}

impl<S: SpadeNum> PointProjection<S> {
    fn new(factor: S, length_2: S) -> Self {
        Self { factor, length_2 }
    }

    pub fn is_before_edge(&self) -> bool {
        self.factor < S::zero()
    }

    pub fn is_after_edge(&self) -> bool {
        self.factor > self.length_2
    }

    pub fn is_on_edge(&self) -> bool {
        !self.is_before_edge() && !self.is_after_edge()
    }

    pub fn reversed(&self) -> Self {
        Self {
            factor: self.length_2 - self.factor,
            length_2: self.length_2,
        }
    }
}

impl<S: SpadeNum + Float> PointProjection<S> {
    pub fn relative_position(&self) -> S {
        self.factor / self.length_2
    }
}

pub fn nearest_point<S>(p1: Point2<S>, p2: Point2<S>, query_point: Point2<S>) -> Point2<S>
where
    S: SpadeNum + Float,
{
    let dir = p2.sub(p1);
    let s = project_point(p1, p2, query_point);
    if s.is_on_edge() {
        let relative_position = s.relative_position();
        p1.add(dir.mul(relative_position))
    } else if s.is_before_edge() {
        p1
    } else {
        p2
    }
}

pub fn project_point<S>(p1: Point2<S>, p2: Point2<S>, query_point: Point2<S>) -> PointProjection<S>
where
    S: SpadeNum,
{
    let dir = p2.sub(p1);
    PointProjection::new(query_point.sub(p1).dot(dir), dir.length2())
}

pub fn distance_2<S>(p1: Point2<S>, p2: Point2<S>, query_point: Point2<S>) -> S
where
    S: SpadeNum + Float,
{
    let nn = nearest_point(p1, p2, query_point);
    query_point.sub(nn).length2()
}

fn to_robust_coord<S: SpadeNum>(point: Point2<S>) -> robust::Coord<S> {
    robust::Coord {
        x: point.x,
        y: point.y,
    }
}

pub fn contained_in_circumference<S>(
    v1: Point2<S>,
    v2: Point2<S>,
    v3: Point2<S>,
    p: Point2<S>,
) -> bool
where
    S: SpadeNum,
{
    let v1 = to_robust_coord(v1);
    let v2 = to_robust_coord(v2);
    let v3 = to_robust_coord(v3);
    let p = to_robust_coord(p);

    // incircle expects all vertices to be ordered CW for right handed systems.
    // For consistency, the public interface of this method will expect the points to be
    // ordered ccw.
    robust::incircle(v3, v2, v1, p) < 0.0
}

pub fn is_ordered_ccw<S>(p1: Point2<S>, p2: Point2<S>, query_point: Point2<S>) -> bool
where
    S: SpadeNum,
{
    let query = side_query(p1, p2, query_point);
    query.is_on_left_side_or_on_line()
}

pub fn side_query<S>(p1: Point2<S>, p2: Point2<S>, query_point: Point2<S>) -> LineSideInfo
where
    S: SpadeNum,
{
    let p1 = to_robust_coord(p1);
    let p2 = to_robust_coord(p2);
    let query_point = to_robust_coord(query_point);

    let result = robust::orient2d(p1, p2, query_point);
    LineSideInfo::from_determinant(result)
}

fn side_query_inaccurate<S>(from: Point2<S>, to: Point2<S>, query_point: Point2<S>) -> LineSideInfo
where
    S: SpadeNum,
{
    let q = query_point;
    let determinant = (to.x - from.x) * (q.y - from.y) - (to.y - from.y) * (q.x - from.x);
    LineSideInfo::from_determinant(determinant.into())
}

pub(crate) fn intersects_edge_non_collinear<S>(
    from0: Point2<S>,
    to0: Point2<S>,
    from1: Point2<S>,
    to1: Point2<S>,
) -> bool
where
    S: SpadeNum,
{
    let other_from = side_query(from0, to0, from1);
    let other_to = side_query(from0, to0, to1);
    let self_from = side_query(from1, to1, from0);
    let self_to = side_query(from1, to1, to0);

    assert!(
        ![&other_from, &other_to, &self_from, &self_to]
            .iter()
            .all(|q| q.is_on_line()),
        "intersects_edge_non_collinear: Given edge is collinear."
    );

    other_from != other_to && self_from != self_to
}

pub fn distance_2_triangle<S>(vertices: [Point2<S>; 3], query_point: Point2<S>) -> S
where
    S: SpadeNum + Float,
{
    for i in 0..3 {
        let from = vertices[i];
        let to = vertices[(i + 1) % 3];
        if side_query_inaccurate(from, to, query_point).is_on_right_side() {
            return distance_2(from, to, query_point);
        }
    }
    // The point lies within the triangle
    S::zero()
}

#[cfg(test)]
mod test {
    use crate::Point2;
    use approx::assert_relative_eq;

    #[test]
    fn test_edge_distance() {
        use super::distance_2;
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(1.0, 1.0);
        assert_relative_eq!(distance_2(p1, p2, Point2::new(1.0, 0.0)), 0.5);
        assert_relative_eq!(distance_2(p1, p2, Point2::new(0.0, 1.)), 0.5);
        assert_relative_eq!(distance_2(p1, p2, Point2::new(-1.0, -1.0)), 2.0);
        assert_relative_eq!(distance_2(p1, p2, Point2::new(2.0, 2.0)), 2.0);
    }

    #[test]
    fn test_edge_side() {
        use super::side_query;

        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(1.0, 1.0);

        assert!(side_query(p1, p2, Point2::new(1.0, 0.0)).is_on_right_side());
        assert!(side_query(p1, p2, Point2::new(0.0, 1.0)).is_on_left_side());
        assert!(side_query(p1, p2, Point2::new(0.5, 0.5)).is_on_line());
    }

    #[test]
    fn test_intersects_middle() {
        use super::intersects_edge_non_collinear;

        let (f0, t0) = (Point2::new(0., 0.), Point2::new(5., 5.0));
        let (f1, t1) = (Point2::new(-1.5, 1.), Point2::new(1.0, -1.5));
        let (f2, t2) = (Point2::new(0.5, 4.), Point2::new(0.5, -4.));

        assert!(!intersects_edge_non_collinear(f0, t0, f1, t1));
        assert!(!intersects_edge_non_collinear(f1, t1, f0, t0));
        assert!(intersects_edge_non_collinear(f0, t0, f2, t2));
        assert!(intersects_edge_non_collinear(f2, t2, f0, t0));
        assert!(intersects_edge_non_collinear(f1, t1, f2, t2));
        assert!(intersects_edge_non_collinear(f2, t2, f1, t1));
    }

    #[test]
    fn test_intersects_end_points() {
        use super::intersects_edge_non_collinear;

        // Check for intersection of one endpoint touching another edge
        let (f1, t1) = (Point2::new(0.33f64, 0.33f64), Point2::new(1.0, 0.0));
        let (f2, t2) = (Point2::new(0.33, -1.0), Point2::new(0.33, 1.0));
        assert!(intersects_edge_non_collinear(f1, t1, f2, t2));
        assert!(intersects_edge_non_collinear(f2, t2, f1, t1));
        let (f3, t3) = (Point2::new(0.0, -1.0), Point2::new(2.0, 1.0));
        assert!(intersects_edge_non_collinear(f1, t1, f3, t3));
        assert!(intersects_edge_non_collinear(f3, t3, f1, t1));

        // Check for intersection if only end points overlap
        let (f4, t4) = (Point2::new(0.33, 0.33), Point2::new(0.0, 2.0));
        assert!(intersects_edge_non_collinear(f1, t1, f4, t4));
        assert!(intersects_edge_non_collinear(f4, t4, f1, t1));
    }

    #[test]
    #[should_panic]
    fn test_collinear_fail() {
        use super::intersects_edge_non_collinear;

        let (f1, t1) = (Point2::new(1.0, 2.0), Point2::new(3.0, 3.0));
        let (f2, t2) = (Point2::new(-1.0, 1.0), Point2::new(-3.0, 0.0));
        intersects_edge_non_collinear(f1, t1, f2, t2);
    }

    #[test]
    fn test_triangle_distance() {
        use super::distance_2_triangle;

        let v1 = Point2::new(0., 0.);
        let v2 = Point2::new(1., 0.);
        let v3 = Point2::new(0., 1.);
        let t = [v1, v2, v3];

        assert_eq!(distance_2_triangle(t, Point2::new(0.25, 0.25)), 0.);
        assert_relative_eq!(distance_2_triangle(t, Point2::new(-1., -1.)), 2.);
        assert_relative_eq!(distance_2_triangle(t, Point2::new(0., -1.)), 1.);
        assert_relative_eq!(distance_2_triangle(t, Point2::new(-1., 0.)), 1.);
        assert_relative_eq!(distance_2_triangle(t, Point2::new(1., 1.)), 0.5);
        assert_relative_eq!(distance_2_triangle(t, Point2::new(0.5, 0.5)), 0.0);
        assert!(distance_2_triangle(t, Point2::new(0.6, 0.6)) > 0.001);
    }

    #[test]
    fn test_contained_in_circumference() {
        use super::contained_in_circumference;

        let (a1, a2, a3) = (3f64, 2f64, 1f64);
        let offset = Point2::new(0.5, 0.7);
        let v1 = Point2::new(a1.sin(), a1.cos()).mul(2.).add(offset);
        let v2 = Point2::new(a2.sin(), a2.cos()).mul(2.).add(offset);
        let v3 = Point2::new(a3.sin(), a3.cos()).mul(2.).add(offset);
        assert!(super::side_query(v1, v2, v3).is_on_left_side());
        assert!(contained_in_circumference(v1, v2, v3, offset));
        let shrunk = (v1.sub(offset)).mul(0.9).add(offset);
        assert!(contained_in_circumference(v1, v2, v3, shrunk));
        let expanded = (v1.sub(offset)).mul(1.1).add(offset);
        assert!(!contained_in_circumference(v1, v2, v3, expanded));
        assert!(!contained_in_circumference(
            v1,
            v2,
            v3,
            Point2::new(2.0 + offset.x, 2.0 + offset.y)
        ));
        assert!(contained_in_circumference(
            Point2::new(0f64, 0f64),
            Point2::new(0f64, -1f64),
            Point2::new(1f64, 0f64),
            Point2::new(0f64, -0.5f64)
        ));
    }
}
