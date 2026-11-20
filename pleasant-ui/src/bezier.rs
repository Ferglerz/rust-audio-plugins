/// Cubic Bézier in logical editor coordinates.
#[derive(Clone, Copy)]
pub struct CubicBezier(pub [(f32, f32); 4]);

impl CubicBezier {
    pub fn point(self, t: f32) -> (f32, f32) {
        let [a, b, c, d] = self.0;
        let u = 1.0 - t;
        let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
        (
            a.0 * weights[0] + b.0 * weights[1] + c.0 * weights[2] + d.0 * weights[3],
            a.1 * weights[0] + b.1 * weights[1] + c.1 * weights[2] + d.1 * weights[3],
        )
    }
}
