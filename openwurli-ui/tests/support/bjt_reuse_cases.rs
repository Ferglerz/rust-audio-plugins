// Frozen v0.9.1 helper from hal0zer0/openwurli c05403b, GPL-3.0-or-later.
// Keep the original arithmetic and unconditional final evaluation as the oracle.
use super::*;

fn original_with_parasitics(
    exits: &mut [usize; 3],
    vbe_ext: f64,
    vbc_ext: f64,
    is: f64,
    vt: f64,
    nf: f64,
    nr: f64,
    beta_f: f64,
    beta_r: f64,
    sign: f64,
    use_gp: bool,
    vaf: f64,
    var: f64,
    ikf: f64,
    ikr: f64,
    ise: f64,
    ne: f64,
    isc: f64,
    nc: f64,
    rb: f64,
    rc: f64,
    re: f64,
) -> (f64, f64, [f64; 4]) {
    const INNER_MAX_ITER: usize = 15;
    const INNER_TOL: f64 = 1e-10;

    // Initial guess: internal = external
    let mut vbe_int = vbe_ext;
    let mut vbc_int = vbc_ext;

    let mut exit = 2;
    for _iter in 0..INNER_MAX_ITER {
        let (ic, ib, jac_int) = bjt_evaluate(
            vbe_int, vbc_int, is, vt, nf, nr, beta_f, beta_r, sign, use_gp, vaf, var, ikf, ikr,
            ise, ne, isc, nc,
        );
        let dic_dvbe = jac_int[0];
        let dic_dvbc = jac_int[1];
        let dib_dvbe = jac_int[2];
        let dib_dvbc = jac_int[3];

        // Residuals
        let f1 = vbe_int - vbe_ext + ib * rb + (ic + ib) * re;
        let f2 = vbc_int - vbc_ext + ib * rb - ic * rc;

        if f1.abs() < INNER_TOL && f2.abs() < INNER_TOL {
            exit = 0;
            break;
        }

        // Jacobian of F w.r.t. (Vbe_int, Vbc_int)
        let j11 = 1.0 + dib_dvbe * rb + (dic_dvbe + dib_dvbe) * re;
        let j12 = dib_dvbc * rb + (dic_dvbc + dib_dvbc) * re;
        let j21 = dib_dvbe * rb - dic_dvbe * rc;
        let j22 = 1.0 + dib_dvbc * rb - dic_dvbc * rc;

        // Solve 2x2 via Cramer's rule
        let det = j11 * j22 - j12 * j21;
        if det.abs() < 1e-30 {
            exit = 1;
            break;
        }
        let inv_det = 1.0 / det;
        let dvbe = (j22 * f1 - j12 * f2) * inv_det;
        let dvbc = (j11 * f2 - j21 * f1) * inv_det;

        // Step limiting: max ±4*VT per iteration
        let max_step = 4.0 * vt;
        let dvbe = dvbe.clamp(-max_step, max_step);
        let dvbc = dvbc.clamp(-max_step, max_step);

        vbe_int -= dvbe;
        vbc_int -= dvbc;
    }

    exits[exit] += 1;

    // Final evaluation at converged internal voltages
    let (ic, ib, jac_int) = bjt_evaluate(
        vbe_int, vbc_int, is, vt, nf, nr, beta_f, beta_r, sign, use_gp, vaf, var, ikf, ikr, ise,
        ne, isc, nc,
    );

    // External Jacobian: J_ext = J_device * J_F^{-1}
    // J_F = [[j11, j12], [j21, j22]] (recompute at converged point)
    let dic_dvbe = jac_int[0];
    let dic_dvbc = jac_int[1];
    let dib_dvbe = jac_int[2];
    let dib_dvbc = jac_int[3];

    let j11 = 1.0 + dib_dvbe * rb + (dic_dvbe + dib_dvbe) * re;
    let j12 = dib_dvbc * rb + (dic_dvbc + dib_dvbc) * re;
    let j21 = dib_dvbe * rb - dic_dvbe * rc;
    let j22 = 1.0 + dib_dvbc * rb - dic_dvbc * rc;

    let det = j11 * j22 - j12 * j21;
    if det.abs() < 1e-30 {
        // Fallback: return intrinsic Jacobian
        return (ic, ib, jac_int);
    }
    let inv_det = 1.0 / det;

    // J_F^{-1} = [[j22, -j12], [-j21, j11]] / det
    let fi11 = j22 * inv_det;
    let fi12 = -j12 * inv_det;
    let fi21 = -j21 * inv_det;
    let fi22 = j11 * inv_det;

    // J_ext = J_device * J_F^{-1}
    let dic_dvbe_ext = dic_dvbe * fi11 + dic_dvbc * fi21;
    let dic_dvbc_ext = dic_dvbe * fi12 + dic_dvbc * fi22;
    let dib_dvbe_ext = dib_dvbe * fi11 + dib_dvbc * fi21;
    let dib_dvbc_ext = dib_dvbe * fi12 + dib_dvbc * fi22;

    (
        ic,
        ib,
        [dic_dvbe_ext, dic_dvbc_ext, dib_dvbe_ext, dib_dvbc_ext],
    )
}

#[test]
fn reuse_matches_original_currents_and_jacobian() {
    let mut exits = [0; 3];
    let mut compare = |vbe, vbc, sign, gp, rb, rc, re| {
        let expected = original_with_parasitics(
            &mut exits, vbe, vbc, 1e-14, 0.02585, 1.0, 1.0, 100.0, 2.0, sign, gp, 100.0, 50.0, 0.1,
            0.05, 1e-15, 1.5, 1e-15, 2.0, rb, rc, re,
        );
        let actual = bjt_with_parasitics(
            vbe, vbc, 1e-14, 0.02585, 1.0, 1.0, 100.0, 2.0, sign, gp, 100.0, 50.0, 0.1, 0.05,
            1e-15, 1.5, 1e-15, 2.0, rb, rc, re,
        );
        for (a, b) in [actual.0, actual.1]
            .into_iter()
            .chain(actual.2)
            .zip([expected.0, expected.1].into_iter().chain(expected.2))
        {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "vbe={vbe}, vbc={vbc}, sign={sign}, gp={gp}, rb={rb}, rc={rc}, re={re}"
            );
        }
    };
    for sign in [-1.0, 1.0] {
        for gp in [false, true] {
            for vbe in [
                -10.0,
                -1.0,
                0.0,
                0.05,
                0.6,
                0.8,
                1.2,
                10.0,
                f64::INFINITY,
                f64::NAN,
            ] {
                for vbc in [-10.0, -0.7, 0.0, 0.1, 0.6, 10.0] {
                    for (rb, rc, re) in [(0.0, 0.0, 0.0), (10.0, 1.0, 0.5), (100.0, 10.0, 2.0)] {
                        compare(vbe, vbc, sign, gp, rb, rc, re);
                    }
                }
            }
        }
    }
    // Deliberately singular inner Jacobian: j11=1, j12=0, j22=0.
    for vbc in [0.1, 0.2, 0.3, 0.4] {
        let (_, _, jac) = bjt_evaluate(
            0.7, vbc, 1e-14, 0.02585, 1.0, 1.0, 100.0, 2.0, 1.0, true, 100.0, 50.0, 0.1, 0.05,
            1e-15, 1.5, 1e-15, 2.0,
        );
        compare(0.7, vbc, 1.0, true, 0.0, 1.0 / jac[1], 0.0);
    }
    assert!(exits[0] > 0, "must exercise tolerance convergence");
    assert!(exits[1] > 0, "must exercise singular-Jacobian exit");
    assert!(exits[2] > 0, "must exercise iteration exhaustion");
}
