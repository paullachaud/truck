use truck_modeling::*;

fn main() {
    let v = builder::vertex(Point3::origin());
    let e = builder::tsweep(&v, Vector3::unit_x());
    let f = builder::tsweep(&e, Vector3::unit_y());
    let cube = builder::tsweep(&f, Vector3::unit_z());

    let v = builder::vertex(Point3::new(0.5, 0.25, -0.5));
    let w = builder::rsweep(&v, Point3::new(0.5, 0.5, 0.0), Vector3::unit_z(), Rad(7.0));
    let f = builder::try_attach_plane(&[w]).unwrap();
    let cylinder = shapeops::not(&builder::tsweep(&f, Vector3::unit_z() * 2.0));
    let and = shapeops::and(&cube, &cylinder);

    let json = serde_json::to_vec_pretty(&and.compress()).unwrap();
    std::fs::write("punched-cube-shapeops.json", &json).unwrap();
}
