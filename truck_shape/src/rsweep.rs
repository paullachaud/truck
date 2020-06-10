use crate::math_impls::*;
use crate::{Builder, Result};
use geometry::*;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::f64::consts::PI;
use topology::*;

pub trait RSweep: Sized {
    type Output: Sized;
    fn partial_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        angle: f64,
        builder: &mut Builder,
    ) -> Result<Self::Output>;
    fn full_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        orientation: bool,
        builder: &mut Builder,
    ) -> Result<Self::Output>;
    fn rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        angle: f64,
        builder: &mut Builder,
    ) -> Result<Self::Output>
    {
        if 2.0 * PI - angle.abs() > Vector::TOLERANCE {
            self.partial_rsweep(origin, axis, angle, builder)
        } else {
            self.full_rsweep(origin, axis, angle > 0.0, builder)
        }
    }
}

impl RSweep for Vertex {
    type Output = Wire;
    fn partial_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        angle: f64,
        builder: &mut Builder,
    ) -> Result<Wire>
    {
        let pt = builder.director.try_get_geometry(&self)?;
        let curve = circle_arc(pt, origin, axis, angle);
        let v = builder.rotated(&self, origin, axis, angle)?;
        let edge = Edge::new_unchecked(self, v);
        builder.director.attach(&edge, curve);
        Ok(Wire::try_from(vec![edge])?)
    }
    fn full_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        orientation: bool,
        builder: &mut Builder,
    ) -> Result<Wire>
    {
        let pt = builder.director.try_get_geometry(&self)?;
        let curve0 = circle_arc(pt, origin, axis, PI);
        let curve1 = circle_arc(pt, origin, axis, -PI);
        let v = builder.rotated(&self, origin, axis, PI)?;
        let edge0 = Edge::new_unchecked(self, v);
        let edge1 = Edge::new_unchecked(self, v);
        builder.director.attach(&edge0, curve0);
        builder.director.attach(&edge1, curve1);
        if orientation {
            Ok(Wire::try_from(vec![edge0, edge1.inverse()])?)
        } else {
            Ok(Wire::try_from(vec![edge1, edge0.inverse()])?)
        }
    }
}

impl RSweep for Edge {
    type Output = Shell;
    fn partial_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        angle: f64,
        builder: &mut Builder,
    ) -> Result<Shell>
    {
        let edge2 = builder.rotated(&self, origin, axis, angle)?;
        let (wire, surface) = sub_partial_sweep_edge(&self, &edge2, origin, axis, angle, builder)?;
        let face = Face::new_unchecked(wire);
        builder.director.attach(&face, surface);
        Ok(vec![face].into())
    }
    fn full_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        orientation: bool,
        builder: &mut Builder,
    ) -> Result<Shell>
    {
        let edge2 = builder.rotated(&self, origin, axis, PI)?;
        let (mut wire0, mut surface0) =
            sub_partial_sweep_edge(&self, &edge2, origin, axis, PI, builder)?;
        let (mut wire1, mut surface1) =
            sub_partial_sweep_edge(&self, &edge2, origin, axis, -PI, builder)?;
        if orientation {
            wire1.inverse();
            surface1.swap_axes();
        } else {
            wire0.inverse();
            surface0.swap_axes();
        }
        let face0 = Face::new_unchecked(wire0);
        let face1 = Face::new_unchecked(wire1);
        builder.director.attach(&face0, surface0);
        builder.director.attach(&face1, surface1);
        Ok(vec![face0, face1].into())
    }
}

fn sub_partial_sweep_edge(
    edge0: &Edge,
    edge2: &Edge,
    origin: &Vector3,
    axis: &Vector3,
    angle: f64,
    builder: &mut Builder,
) -> Result<(Wire, BSplineSurface)>
{
    let pt = builder.director.try_get_geometry(&edge0.back())?;
    let curve1 = circle_arc(&pt, origin, axis, angle);
    let edge1 = Edge::new_unchecked(edge0.back(), edge2.back());
    builder.director.attach(&edge1, curve1);
    let pt = builder.director.try_get_geometry(&edge0.front())?;
    let curve3 = circle_arc(&pt, origin, axis, angle);
    let edge3 = Edge::new_unchecked(edge0.front(), edge2.front());
    builder.director.attach(&edge3, curve3);
    let wire0 = Wire::by_slice(&[*edge0, edge1, edge2.inverse(), edge3.inverse()]);
    let curve0 = builder.director.try_get_geometry(edge0)?;
    let surface0 = rsweep_surface(&curve0, origin, axis, angle);
    Ok((wire0, surface0))
}

impl RSweep for Wire {
    type Output = Shell;
    fn partial_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        angle: f64,
        builder: &mut Builder,
    ) -> Result<Shell>
    {
        let wire = builder.rotated(&self, origin, axis, angle)?;
        connect_by_circle_arc(&self, &wire, origin, axis, angle, builder)
    }
    fn full_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        orientation: bool,
        builder: &mut Builder,
    ) -> Result<Shell>
    {
        let wire = builder.rotated(&self, origin, axis, PI)?;
        let mut shell0 = connect_by_circle_arc(&self, &wire, origin, axis, PI, builder)?;
        let mut shell1 = connect_by_circle_arc(&self, &wire, origin, axis, -PI, builder)?;
        let face_iter = match orientation {
            true => shell1.face_iter_mut(),
            false => shell0.face_iter_mut(),
        };
        for face in face_iter {
            builder.director.reverse_face(face);
        }
        shell0.append(&mut shell1);
        Ok(shell0)
    }
}

fn connect_by_circle_arc(
    wire0: &Wire,
    wire1: &Wire,
    origin: &Vector3,
    axis: &Vector3,
    angle: f64,
    builder: &mut Builder,
) -> Result<Shell>
{
    let mut vemap: HashMap<Vertex, Edge> = HashMap::new();
    for (v0, v1) in wire0.vertex_iter().zip(wire1.vertex_iter()) {
        if vemap.get(&v0).is_none() {
            let pt0 = builder.director.try_get_geometry(&v0)?;
            let curve = circle_arc(&pt0, origin, axis, angle);
            let edge = Edge::new_unchecked(v0, v1);
            builder.director.attach(&edge, curve);
            vemap.insert(v0, edge);
        }
    }
    let mut shell = Shell::new();
    for (edge0, edge2) in wire0.edge_iter().zip(wire1.edge_iter()) {
        let edge1 = vemap.get(&edge0.back()).unwrap();
        let edge3 = vemap.get(&edge0.front()).unwrap();
        let wire1 = Wire::try_from(vec![*edge0, *edge1, edge2.inverse(), edge3.inverse()])?;
        let face = Face::new_unchecked(wire1);
        let curve = builder.director.try_get_geometry(edge0)?;
        let surface = rsweep_surface(curve, origin, axis, angle);
        builder.director.attach(&face, surface);
        shell.push(face);
    }
    Ok(shell)
}

impl RSweep for Face {
    type Output = Solid;
    fn partial_rsweep(
        mut self,
        origin: &Vector3,
        axis: &Vector3,
        angle: f64,
        builder: &mut Builder,
    ) -> Result<Solid>
    {
        let face = builder.rotated(&self, origin, axis, angle)?;
        let mut shell = connect_by_circle_arc(
            self.boundary(),
            face.boundary(),
            origin,
            axis,
            angle,
            builder,
        )?;
        builder.director.reverse_face(&mut self);
        shell.push(self);
        shell.push(face);
        Ok(Solid::new(vec![shell]))
    }
    fn full_rsweep(
        self,
        origin: &Vector3,
        axis: &Vector3,
        orientation: bool,
        builder: &mut Builder,
    ) -> Result<Solid>
    {
        let wire = self.into_boundary();
        let shell = wire.full_rsweep(origin, axis, orientation, builder)?;
        Ok(Solid::new(vec![shell]))
    }
}
