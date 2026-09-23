use pleasant_curves::{s_curve, VelCurve};

fn main() {
    let mut curve = VelCurve::identity();
    curve.insert_at(0.5);
    curve.move_node(1, 0.5, 0.75);
    let serialized = serde_json::to_string(&curve).expect("curve serialization");
    println!(
        "bezier(0.5)={:.3} s-curve(0.5)={:.3} json={serialized}",
        curve.eval_y(0.5),
        s_curve(0.5, 2.5)
    );
}
