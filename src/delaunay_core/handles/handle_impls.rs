use super::super::dcel::EdgeEntry;
use super::super::math;
use super::handle_defs::*;
use super::iterators::CircularIterator;
use super::iterators::NextBackFn;
use super::public_handles::*;
use crate::{HasPosition, LineSideInfo, Point2};
use doc_comment::doc_comment;
use num_traits::{Float, One, Signed};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

// Debug implementations
impl<'a, V, DE, UE, F> std::fmt::Debug for VertexHandle<'a, V, DE, UE, F> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "VertexHandle({:?})", self.handle.index())
    }
}

impl<'a, V, DE, UE, F> Debug for DirectedEdgeHandle<'a, V, DE, UE, F> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(
            f,
            "DirectedEdgeHandle - id: {:?} ({:?} -> {:?})",
            self.handle.index(),
            self.from().fix(),
            self.to().fix()
        )
    }
}

impl<'a, V, DE, UE, F> std::fmt::Debug for UndirectedEdgeHandle<'a, V, DE, UE, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        let [v0, v1] = self.vertices();
        write!(
            f,
            "UndirectedEdgeHandle - id: {:?} ({:?} <-> {:?})",
            self.handle.index(),
            v0.fix(),
            v1.fix(),
        )
    }
}

impl<'a, V, DE, UE, F> std::fmt::Debug for FaceHandle<'a, PossiblyOuterTag, V, DE, UE, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        if let Some(inner) = self.as_inner() {
            inner.fmt(f)
        } else {
            write!(f, "OuterFace")
        }
    }
}

impl<'a, V, DE, UE, F> std::fmt::Debug for FaceHandle<'a, InnerTag, V, DE, UE, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        let [v0, v1, v2] = self.vertices();
        write!(
            f,
            "FaceHandle - id: {:?} ({:?}, {:?}, {:?})",
            self.handle.index(),
            v0.fix().index(),
            v1.fix().index(),
            v2.fix().index(),
        )
    }
}

impl FixedDirectedEdgeHandle {
    #[inline]
    pub(in super::super) fn new_normalized(index: usize) -> Self {
        Self::new(index << 1)
    }

    /// Returns if this edge is the normalized edge of a directed edge pair.
    ///
    /// For every directed edge pair, one edge is marked as the normalized edge. This information
    /// is used to hook up a directed edge handle with it's correct half edge storage.
    #[inline]
    pub(in super::super) fn is_normalized(self) -> bool {
        self.index() & 0x1 == 0x0
    }

    #[inline]
    pub(in super::super) fn normalize_index(self) -> usize {
        self.index() & 0x1
    }

    /// Returns this edge with its direction reversed.
    ///
    /// If this edge points from `v0` to `v1`, the returned edge would point from `v1` to `v0`.
    /// Calling `rev` twice will always return the original vertex.
    #[inline]
    pub fn rev(self) -> Self {
        // Flip the last bit
        Self::new(self.index() ^ 0x1)
    }

    /// Converts this directed edge handle into an undirected edge handle.
    ///
    /// *See also the [handles](crate::handles) module for more information.*
    #[inline]
    pub fn as_undirected(self) -> FixedUndirectedEdgeHandle {
        FixedHandleImpl::new(self.index() >> 1)
    }
}

impl<'a, V, DE, UE, F, Type: Copy, InnerOuter: InnerOuterMarker> Clone
    for DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
    fn clone(&self) -> Self {
        Self {
            dcel: self.dcel,
            handle: self.handle,
        }
    }
}

impl<'a, V, DE, UE, F, Type: Copy, InnerOuter: InnerOuterMarker> Copy
    for DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
}

impl<'a, V, DE, UE, F, Type: PartialEq, InnerOuter: InnerOuterMarker> PartialEq
    for DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<'a, V, DE, UE, F, Type: Eq, InnerOuter: InnerOuterMarker> Eq
    for DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
}

impl<'a, V, DE, UE, F, Type: Hash, InnerOuter: InnerOuterMarker> Hash
    for DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
    fn hash<HA: Hasher>(&self, state: &mut HA) {
        self.handle.hash(state);
    }
}

impl<'a, V, DE, UE, F, Type: Ord, InnerOuter: InnerOuterMarker> Ord
    for DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.handle.cmp(&other.handle)
    }
}

impl<'a, V, DE, UE, F, Type: PartialOrd, InnerOuter: InnerOuterMarker> PartialOrd
    for DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.handle.partial_cmp(&other.handle)
    }
}

impl<'a, V, DE, UE, F, Type: Copy, InnerOuter: InnerOuterMarker>
    DynamicHandleImpl<'a, V, DE, UE, F, Type, InnerOuter>
{
    /// Converts this dynamic handle to its fixed variant.
    ///
    /// *See also the [handles module](index.html).*
    pub fn fix(&self) -> FixedHandleImpl<Type, InnerOuter> {
        self.handle
    }
}

impl FixedFaceHandle<PossiblyOuterTag> {
    /// Returns `true` if this face is the single outer face.
    #[inline]
    pub fn is_outer(&self) -> bool {
        *self == super::super::dcel_operations::OUTER_FACE_HANDLE
    }

    /// Converts this face handle to an inner face.
    ///
    /// Returns `None` if this handle refers to the single outer face.
    pub fn as_inner(&self) -> Option<FixedFaceHandle<InnerTag>> {
        if self.is_outer() {
            None
        } else {
            Some(self.adjust_inner_outer())
        }
    }
}

impl<'a, V, DE, UE, F> AsRef<DE> for DirectedEdgeHandle<'a, V, DE, UE, F> {
    fn as_ref(&self) -> &DE {
        self.data()
    }
}

impl<'a, V, DE, UE, F> DirectedEdgeHandle<'a, V, DE, UE, F> {
    /// Returns the edge's two vertices.
    ///
    /// The first vertex is `self.from()`, the second vertex is `self.to()`.
    pub fn vertices(&self) -> [VertexHandle<'a, V, DE, UE, F>; 2] {
        [self.from(), self.to()]
    }

    /// Returns the edge's origin vertex.
    pub fn from(&self) -> VertexHandle<'a, V, DE, UE, F> {
        let entry = self.dcel.half_edge(self.handle);
        DynamicHandleImpl::new(self.dcel, entry.origin.adjust_inner_outer())
    }

    /// Returns the edges destination vertex.
    pub fn to(&self) -> VertexHandle<'a, V, DE, UE, F> {
        self.rev().from()
    }

    /// Returns this edge in reversed direction.
    #[inline]
    pub fn rev(&self) -> Self {
        DirectedEdgeHandle::new(self.dcel, self.handle.rev())
    }

    /// Returns the vertex which lies opposite of this edge.
    ///
    /// This is equal to calling `self.prev().from()` or `self.next().to()`.
    /// Returns `None` if this edge is part of the convex hull.
    pub fn opposite_vertex(&self) -> Option<VertexHandle<'a, V, DE, UE, F>> {
        if self.is_outer_edge() {
            None
        } else {
            Some(self.prev().from())
        }
    }

    /// Returns the oriented next edge.
    ///
    /// The oriented next edge shares the same face as this edge.
    /// When traversing the face's edges in oriented order,
    /// this edge is the predecessor of the oriented next edge.
    /// "Oriented" means counterclockwise for right handed
    /// coordinate systems.
    pub fn next(&self) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        let entry = self.dcel.half_edge(self.handle);
        DirectedEdgeHandle::new(self.dcel, entry.next)
    }

    /// Returns the oriented previous edge.
    ///
    /// The oriented previous edge shares the same face as this edge.
    /// When traversing the face's edges in oriented order,
    /// this edge is the successor of the oriented previous edge.
    /// "Oriented" means counterclockwise for right handed
    /// coordinate systems.
    pub fn prev(&self) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        let entry = self.dcel.half_edge(self.handle);
        DirectedEdgeHandle::new(self.dcel, entry.prev)
    }

    /// Returns the face located to the left of this edge.
    pub fn face(&self) -> FaceHandle<'a, PossiblyOuterTag, V, DE, UE, F> {
        let entry = self.dcel.half_edge(self.handle);
        self.dcel.face(entry.face)
    }

    /// Returns the next edge in clockwise direction.
    ///
    /// Note that this assumes that you use a right handed coordinate system,
    /// otherwise the sense of orientation is inverted.
    pub fn cw(&self) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        self.rev().next()
    }

    /// Returns the next edge in counter clockwise direction.
    ///
    /// Note that this assumes that you use a right handed coordinate system,
    /// otherwise the sense of orientation is inverted.
    pub fn ccw(&self) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        self.prev().rev()
    }

    /// Returns a reference to the data associated with this directed edge.
    ///
    /// Use [Triangulation::directed_edge_data_mut(handle)](../trait.Triangulation.html#method#directed_edge_data_mut)
    /// to modify the edge data.
    pub fn data(&self) -> &'a DE {
        self.entry().get_directed_data(self.handle)
    }

    fn entry(&self) -> &'a EdgeEntry<DE, UE> {
        self.dcel.edge_entry(self.handle.as_undirected())
    }

    /// Converts this directed edge handle into an undirected edge handle.
    ///
    /// *See also the [handles](crate::handles) module.*
    #[inline]
    pub fn as_undirected(self) -> UndirectedEdgeHandle<'a, V, DE, UE, F> {
        DynamicHandleImpl::new(self.dcel, self.handle.as_undirected())
    }

    /// Returns `true` if this edge is adjacent to the outer face.
    pub fn is_outer_edge(&self) -> bool {
        self.face().is_outer()
    }

    /// Returns `true` if either this edge or its reversed edge is adjacent to the outer face.
    pub fn is_part_of_convex_hull(&self) -> bool {
        self.is_outer_edge() || self.rev().is_outer_edge()
    }

    /// Converts this edge into its corresponding voronoi edge.
    pub fn as_voronoi_edge(&self) -> DirectedVoronoiEdge<'a, V, DE, UE, F> {
        DirectedVoronoiEdge::new(self.dcel, FixedHandleImpl::new(self.handle.index()))
    }
}

impl<'a, V, DE, UE, F> DirectedEdgeHandle<'a, V, DE, UE, F>
where
    V: HasPosition,
{
    /// Returns the position of the vertex opposite of this edge.
    ///
    /// See also [opposite_vertex()](Self::opposite_vertex()).
    /// Returns `None` if this edge is an outer edge.
    #[inline]
    pub fn opposite_position(&self) -> Option<Point2<V::Scalar>> {
        self.opposite_vertex().map(|v| v.position())
    }

    /// Returns the squared length of this edge.
    pub fn length_2(&self) -> V::Scalar {
        self.as_undirected().length_2()
    }

    /// Identifies on which side of this edge a point lies.
    pub fn side_query(&self, query_point: Point2<V::Scalar>) -> LineSideInfo {
        let (p1, p2) = (self.from().position(), self.to().position());
        math::side_query(p1, p2, query_point)
    }

    /// Projects a point on the line going through this edge and returns its relative position.
    ///
    /// This method will return a value between 0. and 1. (linearly interpolated) if the projected
    /// point lies between `self.from` and `self.to`, a value close to zero (due to rounding errors)
    /// if the projected point is equal to `self.from` and a value smaller than zero if the projected
    /// point lies "before" `self.from`. Analogously, a value close to 1. or greater than 1. is
    /// returned if the projected point is equal to or lies behind `self.to`.
    pub fn project_point(
        &self,
        query_point: Point2<V::Scalar>,
    ) -> math::PointProjection<V::Scalar> {
        let (p1, p2) = (self.from().position(), self.to().position());
        math::project_point(p1, p2, query_point)
    }

    pub(crate) fn intersects_edge_non_collinear(
        &self,
        other_from: Point2<V::Scalar>,
        other_to: Point2<V::Scalar>,
    ) -> bool {
        let other_from_query = self.side_query(other_from);
        let other_to_query = self.side_query(other_to);
        let self_from_query = math::side_query(other_from, other_to, self.from().position());
        let self_to_query = math::side_query(other_from, other_to, self.to().position());

        assert!(
            ![
                &other_from_query,
                &other_to_query,
                &self_from_query,
                &self_to_query
            ]
            .iter()
            .all(|q| q.is_on_line()),
            "intersects_edge_non_collinear: Given edge is collinear."
        );

        other_from_query != other_to_query && self_from_query != self_to_query
    }
}

impl FixedUndirectedEdgeHandle {
    /// Converts this directed edge into an undirected edge handle.
    ///
    /// Any of the two directed edges may be returned.
    ///
    /// See also [FixedDirectedEdgeHandle::as_undirected()]()
    ///
    pub fn as_directed(&self) -> FixedDirectedEdgeHandle {
        FixedDirectedEdgeHandle::new_normalized(self.index())
    }

    pub(in super::super) fn normalized(&self) -> FixedDirectedEdgeHandle {
        self.as_directed()
    }

    pub(in super::super) fn not_normalized(&self) -> FixedDirectedEdgeHandle {
        self.as_directed().rev()
    }
}

impl<'a, V, DE, UE, F> UndirectedVoronoiEdge<'a, V, DE, UE, F> {
    /// Returns the edge's two vertices.
    ///
    /// The vertices are returned in any order.
    pub fn vertices(&self) -> [VoronoiVertex<'a, V, DE, UE, F>; 2] {
        [self.as_directed().from(), self.as_directed().to()]
    }

    /// Converts this undirected handle into a directed edge handle.
    pub fn as_directed(&self) -> DirectedVoronoiEdge<'a, V, DE, UE, F> {
        self.as_delaunay_edge().as_directed().as_voronoi_edge()
    }

    /// Returns the corresponding edge of the Delaunay triangulation.
    pub fn as_delaunay_edge(&self) -> UndirectedEdgeHandle<'a, V, DE, UE, F> {
        UndirectedEdgeHandle::new(
            self.dcel,
            FixedUndirectedEdgeHandle::new(self.handle.index()),
        )
    }
}

impl<'a, V, DE, UE, F> AsRef<UE> for UndirectedEdgeHandle<'a, V, DE, UE, F> {
    fn as_ref(&self) -> &UE {
        self.data()
    }
}

impl<'a, V, DE, UE, F> UndirectedEdgeHandle<'a, V, DE, UE, F> {
    /// Returns the edge's two vertices.
    ///
    /// The vertices are returned in any order.
    pub fn vertices(&self) -> [VertexHandle<'a, V, DE, UE, F>; 2] {
        [self.as_directed().from(), self.as_directed().to()]
    }

    /// Converts this directed edge into an undirected edge handle.
    pub fn as_directed(&self) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        DirectedEdgeHandle::new(self.dcel, self.handle.as_directed())
    }

    /// Returns the corresponding edge in the Voronoi diagram.
    pub fn as_voronoi_edge(&self) -> UndirectedVoronoiEdge<'a, V, DE, UE, F> {
        UndirectedVoronoiEdge::new(self.dcel, FixedHandleImpl::new(self.handle.index()))
    }

    /// Returns a reference to the data associated with this directed edge.
    ///
    /// Use [Triangulation::undirected_edge_data_mut(handle)](../trait.Triangulation.html#method#undirected_edge_data_mut)
    /// to modify the edge data.
    pub fn data(&self) -> &UE {
        self.dcel.undirected_edge_data(self.handle)
    }

    /// Returns `true` if the outer face is adjacent to any side of this undirected edge.
    pub fn is_part_of_convex_hull(&self) -> bool {
        self.as_directed().is_part_of_convex_hull()
    }
}

impl<'a, V, DE, UE, F> UndirectedEdgeHandle<'a, V, DE, UE, F>
where
    V: HasPosition,
{
    /// Returns the end positions of this edge.
    ///
    /// The positions are returned in any order.
    pub fn positions(&self) -> [Point2<V::Scalar>; 2] {
        let [v0, v1] = self.vertices();
        [v0.position(), v1.position()]
    }

    /// Returns the squared length of this edge
    pub fn length_2(&self) -> V::Scalar {
        let [p0, p1] = self.positions();
        p0.sub(p1).length2()
    }
}

impl<'a, V, DE, UE, F> UndirectedEdgeHandle<'a, V, DE, UE, F>
where
    V: HasPosition,
    V::Scalar: Float,
{
    /// Returns the squared distance of a point to this edge.
    pub fn distance_2(&self, query_point: Point2<V::Scalar>) -> V::Scalar {
        let [p1, p2] = self.positions();
        math::distance_2(p1, p2, query_point)
    }
}

impl<'a, V, DE, UE, InnerOuter, F> AsRef<F> for FaceHandle<'a, InnerOuter, V, DE, UE, F>
where
    InnerOuter: InnerOuterMarker,
{
    fn as_ref(&self) -> &F {
        self.data()
    }
}

impl<'a, V, DE, UE, F> FaceHandle<'a, InnerTag, V, DE, UE, F> {
    /// Returns the three inner edges adjacent to this face.
    ///
    /// The edges are returned in counter clockwise order.
    pub fn adjacent_edges(&self) -> [DirectedEdgeHandle<'a, V, DE, UE, F>; 3] {
        let e1 = self.adjacent_edge();
        let e0 = e1.prev();
        let e2 = e1.next();
        [e0, e1, e2]
    }

    /// Returns an edge that is adjacent to this face.
    ///
    /// If this face has multiple adjacent edges, any of them is returned.
    pub fn adjacent_edge(&self) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        // unwrap is okay since every inner face has an adjacent edge
        let handle = self.dcel.face_adjacent_edge(self.handle).unwrap();
        DynamicHandleImpl::new(&self.dcel, handle)
    }

    /// Returns the face's three vertices.
    ///
    /// The vertices are returned in counter clockwise order.
    pub fn vertices(&self) -> [VertexHandle<'a, V, DE, UE, F>; 3] {
        let [e0, e1, e2] = self.adjacent_edges();
        [e0.from(), e1.from(), e2.from()]
    }
}

impl<'a, V, DE, UE, F> FaceHandle<'a, InnerTag, V, DE, UE, F>
where
    V: HasPosition,
{
    /// Returns the positions of the face's vertices
    ///
    /// The positions are returned in counter clockwise order.
    pub fn positions(&self) -> [Point2<V::Scalar>; 3] {
        let [v0, v1, v2] = self.vertices();
        [v0.position(), v1.position(), v2.position()]
    }

    /// Returns the triangle's area.
    pub fn area(&self) -> V::Scalar {
        let [v0, v1, v2] = self.positions();
        let b = v1.sub(v0);
        let c = v2.sub(v0);
        (b.x * c.y - b.y * c.x).abs()
    }
}

impl<'a, V, DE, UE, F> FaceHandle<'a, InnerTag, V, DE, UE, F>
where
    V: HasPosition,
    V::Scalar: Float,
{
    /// Returns the squared distance of a point to this triangle.
    ///
    /// The distance of a point inside the triangle is zero.
    pub fn distance_2(&self, query_point: Point2<V::Scalar>) -> V::Scalar {
        math::distance_2_triangle(self.positions(), query_point)
    }

    /// Returns the face's center point.
    ///
    /// The center point is the average position.
    pub fn center(&self) -> Point2<V::Scalar> {
        let [v0, v1, v2] = self.positions();
        let one = V::Scalar::one();
        let three = one + one + one;
        v0.add(v1.add(v2)).mul(one / three)
    }
    /// Returns the face's circumcircle center and the **squared** radius of the circumcircle.
    ///
    /// The circumcircle is the unique circle that intersects all three vertices of the face.
    pub fn circumcircle(&self) -> (Point2<V::Scalar>, V::Scalar) {
        let [v0, v1, v2] = self.positions();
        let b = v1.sub(v0);
        let c = v2.sub(v0);

        let one = V::Scalar::one();
        let two = one + one;
        let d = two * (b.x * c.y - c.x * b.y);
        let len_b = b.dot(b);
        let len_c = c.dot(c);
        let d_inv: V::Scalar = one / d;

        let x = (len_b * c.y - len_c * b.y) * d_inv;
        let y = (-len_b * c.x + len_c * b.x) * d_inv;
        let result = Point2::new(x, y);
        (result.add(v0), x * x + y * y)
    }

    /// Returns the face's circumcenter.
    ///
    /// The circumcenter is the center of the circumcircle.
    pub fn circumcenter(&self) -> Point2<V::Scalar> {
        self.circumcircle().0
    }

    /// Returns the barycentric coordinates of a point relative to this face.
    ///
    /// The returned coordinates will sum up to 1.
    pub fn barycentric_interpolation(&self, coordinate: Point2<V::Scalar>) -> [V::Scalar; 3] {
        let [v1, v2, v3] = self.vertices();
        let [v1, v2, v3] = [v1.position(), v2.position(), v3.position()];
        let (x, y) = (coordinate.x, coordinate.y);
        let (x1, x2, x3) = (v1.x, v2.x, v3.x);
        let (y1, y2, y3) = (v1.y, v2.y, v3.y);
        let det = (y2 - y3) * (x1 - x3) + (x3 - x2) * (y1 - y3);
        let lambda1 = ((y2 - y3) * (x - x3) + (x3 - x2) * (y - y3)) / det;
        let lambda2 = ((y3 - y1) * (x - x3) + (x1 - x3) * (y - y3)) / det;
        let lambda3 = V::Scalar::one() - lambda1 - lambda2;
        [lambda1, lambda2, lambda3]
    }
}

impl<'a, V, DE, UE, F> AsRef<V> for VertexHandle<'a, V, DE, UE, F> {
    fn as_ref(&self) -> &V {
        self.data()
    }
}

impl<'a, V, DE, UE, F> VertexHandle<'a, V, DE, UE, F>
where
    V: HasPosition,
{
    /// Returns the position of this vertex.
    pub fn position(&self) -> Point2<V::Scalar> {
        self.dcel.vertex_data(self.handle).position()
    }
}

pub struct CCWEdgesNextBackFn;

impl NextBackFn for CCWEdgesNextBackFn {
    fn next<'a, V, DE, UE, F>(
        edge_handle: DirectedEdgeHandle<'a, V, DE, UE, F>,
    ) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        edge_handle.ccw()
    }

    fn next_back<'a, V, DE, UE, F>(
        edge_handle: DirectedEdgeHandle<'a, V, DE, UE, F>,
    ) -> DirectedEdgeHandle<'a, V, DE, UE, F> {
        edge_handle.cw()
    }
}

impl<'a, V, DE, UE, F> VertexHandle<'a, V, DE, UE, F> {
    doc_comment! {
    concat!(
"Returns all directed edges going out of this vertex.

The edges are returned in counter clockwise order, beginning at an arbitrary
edge.\n\n",
    include_str!("../../../images/circular_iterator.svg"),
"\n\n *A possible iteration order of `v.out_edges()`*\n

*Note*: The returned iterator implements `DoubleEndedIterator`, allowing traversal in
clockwise order."),
    pub fn out_edges(&self) -> CircularIterator<'a, V, DE, UE, F, CCWEdgesNextBackFn> {
        if let Some(edge) = self.out_edge() {
            CircularIterator::new(edge)
        } else {
            CircularIterator::new_empty(DirectedEdgeHandle::new(
                self.dcel,
                FixedDirectedEdgeHandle::new(0),
            ))
        }
    }
    }

    /// Returns an outgoing edge of this vertex.
    ///
    /// If the vertex has multiple outgoing edges, any of them is returned.
    pub fn out_edge(&self) -> Option<DirectedEdgeHandle<'a, V, DE, UE, F>> {
        self.dcel
            .vertex_out_edge(self.handle)
            .map(|handle| DirectedEdgeHandle::new(self.dcel, handle))
    }

    /// Returns the data associated with this vertex.
    pub fn data(&self) -> &V {
        self.dcel.vertex_data(self.handle)
    }

    /// Returns the voronoi face that corresponds to this vertex of the Delaunay triangulation.
    pub fn as_voronoi_face(&self) -> VoronoiFace<'a, V, DE, UE, F> {
        VoronoiFace::new(self.dcel, FixedHandleImpl::new(self.handle.index()))
    }
}

impl<'a, V, DE, UE, F> DirectedEdgeHandle<'a, V, DE, UE, F>
where
    V: HasPosition,
    V::Scalar: Float,
{
    /// Returns the squared distance of a point to this edge.
    pub fn distance_2(&self, query_point: Point2<V::Scalar>) -> V::Scalar {
        self.as_undirected().distance_2(query_point)
    }

    /// Yields the nearest point on this edge.
    pub fn nearest_point(&self, query_point: Point2<V::Scalar>) -> Point2<V::Scalar> {
        let (p1, p2) = (self.from().position(), self.to().position());
        math::nearest_point(p1, p2, query_point)
    }
}

impl<'a, V, DE, UE, F, InnerOuter: InnerOuterMarker> FaceHandle<'a, InnerOuter, V, DE, UE, F> {
    /// Returns a reference to the data associated with this face.
    pub fn data(&self) -> &F {
        self.dcel.face_data(self.handle)
    }
}

impl<'a, V, DE, UE, F> FaceHandle<'a, PossiblyOuterTag, V, DE, UE, F> {
    /// Returns `true` if this handle refers to the single outer face.
    #[inline]
    pub fn is_outer(&self) -> bool {
        self.handle.is_outer()
    }

    /// Converts this possibly outer face handle face handle into an inner face handle.
    ///
    /// Returns `None` if this handle refers to the outer face.
    pub fn as_inner(&self) -> Option<FaceHandle<'a, InnerTag, V, DE, UE, F>> {
        if self.is_outer() {
            None
        } else {
            Some(FaceHandle::new(self.dcel, self.handle.adjust_inner_outer()))
        }
    }

    /// Returns an edge that is adjacent to this face.
    ///
    /// The returned edge has this face on its left side.
    /// Returns `None` if the triangulation has only one or none vertices.
    pub fn adjacent_edge(&self) -> Option<DirectedEdgeHandle<'a, V, DE, UE, F>> {
        self.dcel
            .face_adjacent_edge(self.handle)
            .map(|handle| DirectedEdgeHandle::new(&self.dcel, handle))
    }
}

#[cfg(test)]
mod test {
    use super::FixedDirectedEdgeHandle;

    #[test]
    fn test_new_normalized_and_index_and_sym() {
        for index in 0..10 {
            let handle: FixedDirectedEdgeHandle = FixedDirectedEdgeHandle::new_normalized(index);
            let rev = handle.rev();
            assert_eq!(handle.as_undirected().index(), index);
            assert!(handle.is_normalized());

            assert_ne!(handle, handle.rev());
            assert!(!rev.is_normalized());
            assert_eq!(rev.rev(), handle);
        }
    }
}
