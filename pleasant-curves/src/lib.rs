//! Host-independent curve geometry and evaluation.

pub mod analytic;
pub mod bezier;

pub use analytic::{inv_s_curve, s_curve};
pub use bezier::{VelCurve, VelHandle, VelNode, MAX_NODES};
