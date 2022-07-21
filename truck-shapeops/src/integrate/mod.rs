use crate::*;
use truck_geometry::IntersectionCurve;
use truck_meshalgo::prelude::*;
use truck_topology::*;

/// Only solids consisting of faces whose surface is implemented this trait can be used for set operations.
pub trait ShapeOpsSurface:
    ParametricSurface3D
    + ParameterDivision2D
    + SearchParameter<D2, Point = Point3>
    + SearchNearestParameter<D2, Point = Point3>
    + Invertible {
}
impl<S> ShapeOpsSurface for S where S: ParametricSurface3D
        + ParameterDivision2D
        + SearchParameter<D2, Point = Point3>
        + SearchNearestParameter<D2, Point = Point3>
        + Invertible
{
}

/// Only solids consisting of edges whose curve is implemented this trait can be used for set operations.
pub trait ShapeOpsCurve<S: ShapeOpsSurface>:
    ParametricCurve3D
    + ParameterDivision1D<Point = Point3>
    + Cut
    + Invertible
    + From<IntersectionCurve<PolylineCurve<Point3>, S>>
    + SearchParameter<D1, Point = Point3>
    + SearchNearestParameter<D1, Point = Point3> {
}
impl<C, S: ShapeOpsSurface> ShapeOpsCurve<S> for C where C: ParametricCurve3D
        + ParameterDivision1D<Point = Point3>
        + Cut
        + Invertible
        + From<IntersectionCurve<PolylineCurve<Point3>, S>>
        + SearchParameter<D1, Point = Point3>
        + SearchNearestParameter<D1, Point = Point3>
{
}

fn process_one_pair_of_shells<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    shell0: &Shell<Point3, C, S>,
    shell1: &Shell<Point3, C, S>,
    tol: f64,
) -> Option<[Shell<Point3, C, S>; 2]> {
    nonpositive_tolerance!(tol);
    let poly_shell0 = shell0.triangulation(tol);
    let poly_shell1 = shell1.triangulation(tol);
    let loops_store::LoopsStoreQuadruple {
        geom_loops_store0: loops_store0,
        geom_loops_store1: loops_store1,
        ..
    } = loops_store::create_loops_stores(shell0, &poly_shell0, shell1, &poly_shell1, tol)?;
    let mut cls0 = divide_face::divide_faces(shell0, &loops_store0, tol)?;
    cls0.integrate_by_component();
    let mut cls1 = divide_face::divide_faces(shell1, &loops_store1, tol)?;
    cls1.integrate_by_component();
    let [mut and0, mut or0, unknown0] = cls0.and_or_unknown();
    unknown0.into_iter().try_for_each(|face| {
        let pt = face.boundaries()[0]
            .vertex_iter()
            .next()
            .unwrap()
            .get_point();
        let dir = hash::take_one_unit(pt);
        let count = poly_shell1.iter().try_fold(0, |count, face| {
            let poly = face.get_surface()?;
            Some(count + poly.signed_crossing_faces(pt, dir))
        })?;
        if count >= 1 {
            and0.push(face);
        } else {
            or0.push(face);
        }
        Some(())
    })?;
    let [mut and1, mut or1, unknown1] = cls1.and_or_unknown();
    unknown1.into_iter().try_for_each(|face| {
        let pt = face.boundaries()[0]
            .vertex_iter()
            .next()
            .unwrap()
            .get_point();
        let dir = hash::take_one_unit(pt);
        let count = poly_shell0.iter().try_fold(0, |count, face| {
            let poly = face.get_surface()?;
            Some(count + poly.signed_crossing_faces(pt, dir))
        })?;
        if count >= 1 {
            and1.push(face);
        } else {
            or1.push(face);
        }
        Some(())
    })?;
    and0.append(&mut and1);
    or0.append(&mut or1);
    Some([and0, or0])
}

/// AND operation between two solids.
pub fn and<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> Option<Solid<Point3, C, S>> {
    let mut iter0 = solid0.boundaries().iter();
    let mut iter1 = solid1.boundaries().iter();
    let shell0 = iter0.next().unwrap();
    let shell1 = iter1.next().unwrap();
    let [mut and_shell, _] = process_one_pair_of_shells(shell0, shell1, tol)?;
    for shell in iter0 {
        let [res, _] = process_one_pair_of_shells(&and_shell, shell, tol)?;
        and_shell = res;
    }
    for shell in iter1 {
        let [res, _] = process_one_pair_of_shells(&and_shell, shell, tol)?;
        and_shell = res;
    }
    let boundaries = and_shell.connected_components();
    Some(Solid::new(boundaries))
}

/// OR operation between two solids.
pub fn or<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> Option<Solid<Point3, C, S>> {
    let mut iter0 = solid0.boundaries().iter();
    let mut iter1 = solid1.boundaries().iter();
    let shell0 = iter0.next().unwrap();
    let shell1 = iter1.next().unwrap();
    let [_, mut or_shell] = process_one_pair_of_shells(shell0, shell1, tol)?;
    for shell in iter0 {
        let [_, res] = process_one_pair_of_shells(&or_shell, shell, tol)?;
        or_shell = res;
    }
    for shell in iter1 {
        let [_, res] = process_one_pair_of_shells(&or_shell, shell, tol)?;
        or_shell = res;
    }
    let boundaries = or_shell.connected_components();
    Some(Solid::new(boundaries))
}

#[cfg(test)]
mod tests;
