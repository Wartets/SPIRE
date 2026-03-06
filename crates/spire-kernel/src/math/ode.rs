//! # ODE Solvers - Generic Ordinary Differential Equation Integration
//!
//! This module provides reusable, high-precision numerical solvers for
//! systems of first-order ordinary differential equations of the form:
//!
//! $$\frac{dy}{dx} = f(x, y)$$
//!
//! Both scalar ($y \in \mathbb{R}$) and vector ($y \in \mathbb{R}^n$)
//! systems are supported through the generic [`OdeSolver`] trait.
//!
//! ## Solvers
//!
//! - [`RungeKutta4`] - Classical 4th-order Runge-Kutta with fixed step
//!   size.  Excellent accuracy for smooth problems with predictable
//!   step requirements.
//!
//! - [`DormandPrince45`] - Adaptive Runge-Kutta 4(5) (Dormand-Prince)
//!   with embedded error estimation and automatic step-size control.
//!   Essential for stiff transitions such as cosmological freeze-out.
//!
//! ## Usage
//!
//! ```rust
//! use spire_kernel::math::ode::{RungeKutta4, DormandPrince45, OdeSolver};
//!
//! // Solve dy/dx = -y  (exponential decay y = e^{-x})
//! let rk4 = RungeKutta4::new(0.01);
//! let solution = rk4.integrate(|_x, y| -y, 0.0, 1.0, 1.0);
//! let final_y = solution.last().unwrap().1;
//! assert!((final_y - (-1.0_f64).exp()).abs() < 1e-6);
//!
//! // Adaptive solver with automatic step sizing
//! let dp = DormandPrince45::new(1e-8, 1e-8);
//! let solution = dp.integrate(|_x, y| -y, 0.0, 1.0, 5.0);
//! let final_y = solution.last().unwrap().1;
//! assert!((final_y - (-5.0_f64).exp()).abs() < 1e-6);
//! ```

use serde::{Deserialize, Serialize};

// ===========================================================================
// ODE Solver Trait
// ===========================================================================

/// Result point from an ODE integration: $(x_i, y_i)$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdePoint {
    /// The independent variable value.
    pub x: f64,
    /// The dependent variable value at this point.
    pub y: f64,
}

/// Generic interface for scalar ODE solvers.
///
/// All solvers take an equation $dy/dx = f(x, y)$, initial conditions
/// $(x_0, y_0)$, and an endpoint $x_f$, returning a trajectory of
/// $(x, y)$ pairs from $x_0$ to $x_f$.
pub trait OdeSolver {
    /// Integrate the scalar ODE $dy/dx = f(x, y)$ from $x_0$ to $x_f$.
    ///
    /// # Arguments
    /// * `f` - The right-hand side function $f(x, y)$.
    /// * `x0` - Initial value of the independent variable.
    /// * `y0` - Initial value of the dependent variable.
    /// * `xf` - Final value of the independent variable.
    ///
    /// # Returns
    /// A vector of `(x, y)` pairs tracing the solution trajectory.
    fn integrate<F>(&self, f: F, x0: f64, y0: f64, xf: f64) -> Vec<(f64, f64)>
    where
        F: Fn(f64, f64) -> f64;
}

/// Generic interface for vector ODE solvers ($y \in \mathbb{R}^n$).
///
/// Solves systems of coupled first-order ODEs:
/// $$\frac{d\mathbf{y}}{dx} = \mathbf{f}(x, \mathbf{y})$$
pub trait VectorOdeSolver {
    /// Integrate a system of coupled ODEs from $x_0$ to $x_f$.
    ///
    /// # Arguments
    /// * `f` - The right-hand side function $\mathbf{f}(x, \mathbf{y})$.
    /// * `x0` - Initial value of the independent variable.
    /// * `y0` - Initial state vector.
    /// * `xf` - Final value of the independent variable.
    ///
    /// # Returns
    /// A vector of `(x, y_vec)` pairs tracing the solution trajectory.
    fn integrate_system<F>(&self, f: F, x0: f64, y0: &[f64], xf: f64) -> Vec<(f64, Vec<f64>)>
    where
        F: Fn(f64, &[f64]) -> Vec<f64>;
}

// ===========================================================================
// Classical Runge-Kutta 4th Order (RK4)
// ===========================================================================

/// Classical 4th-order Runge-Kutta integrator with fixed step size.
///
/// Uses the well-known four-stage Butcher tableau:
///
/// $$y_{n+1} = y_n + \frac{h}{6}(k_1 + 2k_2 + 2k_3 + k_4)$$
///
/// where $k_1 = f(x_n, y_n)$, $k_2 = f(x_n + h/2, y_n + hk_1/2)$, etc.
///
/// This is an excellent general-purpose solver for smooth problems.
/// For stiff systems or problems requiring high precision with varying
/// scales, prefer [`DormandPrince45`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RungeKutta4 {
    /// Fixed step size $h$.
    pub step_size: f64,
}

impl RungeKutta4 {
    /// Create a new RK4 solver with the specified fixed step size.
    pub fn new(step_size: f64) -> Self {
        Self { step_size }
    }

    /// Perform a single RK4 step.
    ///
    /// Returns the new value $y_{n+1}$ after advancing from $x_n$ by step $h$.
    pub fn step<F>(&self, f: &F, x: f64, y: f64, h: f64) -> f64
    where
        F: Fn(f64, f64) -> f64,
    {
        let k1 = f(x, y);
        let k2 = f(x + 0.5 * h, y + 0.5 * h * k1);
        let k3 = f(x + 0.5 * h, y + 0.5 * h * k2);
        let k4 = f(x + h, y + h * k3);
        y + (h / 6.0) * (k1 + 2.0 * k2 + 2.0 * k3 + k4)
    }
}

impl OdeSolver for RungeKutta4 {
    fn integrate<F>(&self, f: F, x0: f64, y0: f64, xf: f64) -> Vec<(f64, f64)>
    where
        F: Fn(f64, f64) -> f64,
    {
        let direction = if xf >= x0 { 1.0 } else { -1.0 };
        let h = direction * self.step_size.abs();
        let n_steps = ((xf - x0) / h).abs().ceil() as usize;

        let mut trajectory = Vec::with_capacity(n_steps + 1);
        let mut x = x0;
        let mut y = y0;
        trajectory.push((x, y));

        for i in 0..n_steps {
            let h_actual = if i == n_steps - 1 {
                xf - x // exact landing on the endpoint
            } else {
                h
            };
            y = self.step(&f, x, y, h_actual);
            x += h_actual;
            trajectory.push((x, y));
        }

        trajectory
    }
}

impl VectorOdeSolver for RungeKutta4 {
    fn integrate_system<F>(&self, f: F, x0: f64, y0: &[f64], xf: f64) -> Vec<(f64, Vec<f64>)>
    where
        F: Fn(f64, &[f64]) -> Vec<f64>,
    {
        let direction = if xf >= x0 { 1.0 } else { -1.0 };
        let h_base = direction * self.step_size.abs();
        let n_steps = ((xf - x0) / h_base).abs().ceil() as usize;
        let dim = y0.len();

        let mut trajectory = Vec::with_capacity(n_steps + 1);
        let mut x = x0;
        let mut y: Vec<f64> = y0.to_vec();
        trajectory.push((x, y.clone()));

        for i in 0..n_steps {
            let h = if i == n_steps - 1 { xf - x } else { h_base };

            let k1 = f(x, &y);

            let y_tmp: Vec<f64> = (0..dim).map(|j| y[j] + 0.5 * h * k1[j]).collect();
            let k2 = f(x + 0.5 * h, &y_tmp);

            let y_tmp: Vec<f64> = (0..dim).map(|j| y[j] + 0.5 * h * k2[j]).collect();
            let k3 = f(x + 0.5 * h, &y_tmp);

            let y_tmp: Vec<f64> = (0..dim).map(|j| y[j] + h * k3[j]).collect();
            let k4 = f(x + h, &y_tmp);

            for j in 0..dim {
                y[j] += (h / 6.0) * (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]);
            }
            x += h;
            trajectory.push((x, y.clone()));
        }

        trajectory
    }
}

// ===========================================================================
// Dormand-Prince 4(5) Adaptive Solver (RK45)
// ===========================================================================

/// Adaptive Runge-Kutta 4(5) solver using the Dormand-Prince method.
///
/// Provides automatic step-size control by computing both a 4th-order
/// and a 5th-order estimate at each step. The local truncation error
/// drives the step-size adaptation:
///
/// $$h_{\text{new}} = h_{\text{old}} \cdot \min\!\left(f_{\max},
///   \max\!\left(f_{\min}, S \left(\frac{\varepsilon}{|e|}\right)^{1/5}
///   \right)\right)$$
///
/// where $\varepsilon$ is the tolerance, $|e|$ is the estimated error,
/// and $S = 0.9$ is the safety factor.
///
/// This is the method underlying MATLAB's `ode45` and SciPy's
/// `solve_ivp(method='RK45')`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DormandPrince45 {
    /// Absolute error tolerance.
    pub atol: f64,
    /// Relative error tolerance.
    pub rtol: f64,
    /// Maximum number of steps before aborting.
    pub max_steps: usize,
    /// Initial step size (if zero, estimated automatically).
    pub initial_step: f64,
    /// Minimum allowed step size.
    pub min_step: f64,
    /// Maximum allowed step size.
    pub max_step: f64,
}

impl DormandPrince45 {
    /// Create a new Dormand-Prince solver with the given tolerances.
    ///
    /// Step-size bounds and initial step are set to sensible defaults.
    pub fn new(atol: f64, rtol: f64) -> Self {
        Self {
            atol,
            rtol,
            max_steps: 100_000,
            initial_step: 0.0,
            min_step: 1e-15,
            max_step: 1e6,
        }
    }

    /// Create a solver with full configuration control.
    pub fn with_config(
        atol: f64,
        rtol: f64,
        max_steps: usize,
        initial_step: f64,
        min_step: f64,
        max_step: f64,
    ) -> Self {
        Self {
            atol,
            rtol,
            max_steps,
            initial_step,
            min_step,
            max_step,
        }
    }

    /// Dormand-Prince Butcher tableau coefficients.
    #[allow(clippy::type_complexity)]
    fn dp_step<F>(&self, f: &F, x: f64, y: f64, h: f64) -> (f64, f64, f64)
    where
        F: Fn(f64, f64) -> f64,
    {
        // Dormand-Prince coefficients
        let k1 = h * f(x, y);
        let k2 = h * f(x + (1.0 / 5.0) * h, y + (1.0 / 5.0) * k1);
        let k3 = h * f(
            x + (3.0 / 10.0) * h,
            y + (3.0 / 40.0) * k1 + (9.0 / 40.0) * k2,
        );
        let k4 = h * f(
            x + (4.0 / 5.0) * h,
            y + (44.0 / 45.0) * k1 - (56.0 / 15.0) * k2 + (32.0 / 9.0) * k3,
        );
        let k5 = h * f(
            x + (8.0 / 9.0) * h,
            y + (19372.0 / 6561.0) * k1 - (25360.0 / 2187.0) * k2 + (64448.0 / 6561.0) * k3
                - (212.0 / 729.0) * k4,
        );
        let k6 = h * f(
            x + h,
            y + (9017.0 / 3168.0) * k1 - (355.0 / 33.0) * k2
                + (46732.0 / 5247.0) * k3
                + (49.0 / 176.0) * k4
                - (5103.0 / 18656.0) * k5,
        );

        // 5th order solution
        let y5 = y + (35.0 / 384.0) * k1 + (500.0 / 1113.0) * k3 + (125.0 / 192.0) * k4
            - (2187.0 / 6784.0) * k5
            + (11.0 / 84.0) * k6;

        // 4th order solution (for error estimate)
        let k7 = h * f(x + h, y5);
        let y4 = y + (5179.0 / 57600.0) * k1 + (7571.0 / 16695.0) * k3 + (393.0 / 640.0) * k4
            - (92097.0 / 339200.0) * k5
            + (187.0 / 2100.0) * k6
            + (1.0 / 40.0) * k7;

        let error = (y5 - y4).abs();
        (y5, y4, error)
    }
}

impl OdeSolver for DormandPrince45 {
    fn integrate<F>(&self, f: F, x0: f64, y0: f64, xf: f64) -> Vec<(f64, f64)>
    where
        F: Fn(f64, f64) -> f64,
    {
        let direction = if xf >= x0 { 1.0 } else { -1.0 };
        let span = (xf - x0).abs();

        // Estimate initial step size
        let mut h = if self.initial_step > 0.0 {
            direction * self.initial_step.min(span)
        } else {
            direction * (span * 0.001).max(self.min_step).min(self.max_step)
        };

        let safety = 0.9_f64;
        let grow_max = 5.0_f64;
        let shrink_min = 0.2_f64;

        let mut trajectory = Vec::with_capacity(1024);
        let mut x = x0;
        let mut y = y0;
        trajectory.push((x, y));

        for _ in 0..self.max_steps {
            if direction * (xf - x) <= 0.0 {
                break;
            }

            // Don't overshoot the endpoint
            if direction * (x + h - xf) > 0.0 {
                h = xf - x;
            }

            let (y5, _y4, error) = self.dp_step(&f, x, y, h);

            // Compute tolerance
            let tol = self.atol + self.rtol * y.abs().max(y5.abs());

            if error <= tol || h.abs() <= self.min_step {
                // Accept step
                x += h;
                y = y5;
                trajectory.push((x, y));

                if direction * (xf - x) <= 0.0 {
                    break;
                }

                // Grow step for next iteration
                if error > 0.0 {
                    let factor = safety * (tol / error).powf(0.2);
                    h *= factor.min(grow_max).max(shrink_min);
                } else {
                    h *= grow_max;
                }

                // Clamp to bounds
                h = direction * h.abs().max(self.min_step).min(self.max_step);
            } else {
                // Reject step and shrink
                let factor = safety * (tol / error).powf(0.25);
                h *= factor.max(shrink_min);
                h = direction * h.abs().max(self.min_step).min(self.max_step);
            }
        }

        trajectory
    }
}

// ===========================================================================
// Convenience Functions
// ===========================================================================

/// Solve a scalar ODE using the classical RK4 method with fixed step size.
///
/// This is a convenience wrapper around [`RungeKutta4`].
pub fn solve_ode_rk4<F>(f: F, x0: f64, y0: f64, xf: f64, step_size: f64) -> Vec<(f64, f64)>
where
    F: Fn(f64, f64) -> f64,
{
    RungeKutta4::new(step_size).integrate(f, x0, y0, xf)
}

/// Solve a scalar ODE using the adaptive Dormand-Prince method.
///
/// This is a convenience wrapper around [`DormandPrince45`].
pub fn solve_ode_adaptive<F>(
    f: F,
    x0: f64,
    y0: f64,
    xf: f64,
    atol: f64,
    rtol: f64,
) -> Vec<(f64, f64)>
where
    F: Fn(f64, f64) -> f64,
{
    DormandPrince45::new(atol, rtol).integrate(f, x0, y0, xf)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── RK4 Tests ────────────────────────────────────────────────────────

    #[test]
    fn rk4_exponential_decay() {
        // dy/dx = -y, y(0) = 1  =>  y = e^{-x}
        let solver = RungeKutta4::new(0.001);
        let traj = solver.integrate(|_x, y| -y, 0.0, 1.0, 5.0);

        let (x_final, y_final) = traj.last().unwrap();
        assert!((x_final - 5.0).abs() < 1e-10, "Should reach x=5");
        let exact = (-5.0_f64).exp();
        assert!(
            (y_final - exact).abs() < 1e-6,
            "RK4 exp decay: got {}, expected {}",
            y_final,
            exact
        );
    }

    #[test]
    fn rk4_linear_growth() {
        // dy/dx = 2x, y(0) = 0  =>  y = x^2
        let solver = RungeKutta4::new(0.01);
        let traj = solver.integrate(|x, _y| 2.0 * x, 0.0, 0.0, 3.0);

        let (x_final, y_final) = traj.last().unwrap();
        assert!((x_final - 3.0).abs() < 1e-10);
        assert!(
            (y_final - 9.0).abs() < 1e-6,
            "RK4 quadratic: got {}, expected 9.0",
            y_final
        );
    }

    #[test]
    fn rk4_sinusoidal() {
        // dy/dx = cos(x), y(0) = 0  =>  y = sin(x)
        let solver = RungeKutta4::new(0.001);
        let traj = solver.integrate(|x, _y| x.cos(), 0.0, 0.0, std::f64::consts::PI);

        let (_, y_final) = traj.last().unwrap();
        // sin(pi) = 0
        assert!(
            y_final.abs() < 1e-6,
            "RK4 sin: got {}, expected ~0",
            y_final
        );
    }

    #[test]
    fn rk4_negative_direction() {
        // Integrate backwards: dy/dx = -y from x=5 to x=0, y(5) = e^{-5}
        let solver = RungeKutta4::new(0.001);
        let y_start = (-5.0_f64).exp();
        let traj = solver.integrate(|_x, y| -y, 5.0, y_start, 0.0);

        let (x_final, y_final) = traj.last().unwrap();
        assert!((x_final - 0.0).abs() < 1e-10);
        assert!(
            (y_final - 1.0).abs() < 1e-4,
            "RK4 reverse: got {}, expected 1.0",
            y_final
        );
    }

    #[test]
    fn rk4_single_step_accuracy() {
        // One step of RK4 on dy/dx = y (exponential growth)
        let solver = RungeKutta4::new(0.1);
        let y1 = solver.step(&|_x: f64, y: f64| y, 0.0, 1.0, 0.1);
        let exact = (0.1_f64).exp();
        // RK4 local error is O(h^5) = O(1e-5)
        assert!(
            (y1 - exact).abs() < 1e-5,
            "Single RK4 step: got {}, expected {}",
            y1,
            exact
        );
    }

    // ── Dormand-Prince Adaptive Tests ────────────────────────────────────

    #[test]
    fn dp45_exponential_decay() {
        let solver = DormandPrince45::new(1e-10, 1e-10);
        let traj = solver.integrate(|_x, y| -y, 0.0, 1.0, 10.0);

        let (x_final, y_final) = traj.last().unwrap();
        assert!((x_final - 10.0).abs() < 1e-8);
        let exact = (-10.0_f64).exp();
        assert!(
            (y_final - exact).abs() / exact.abs() < 1e-5,
            "DP45 exp decay: got {}, expected {}",
            y_final,
            exact
        );
    }

    #[test]
    fn dp45_polynomial() {
        // dy/dx = 3x^2, y(0) = 0  =>  y = x^3
        let solver = DormandPrince45::new(1e-10, 1e-10);
        let traj = solver.integrate(|x, _y| 3.0 * x * x, 0.0, 0.0, 4.0);

        let (_, y_final) = traj.last().unwrap();
        assert!(
            (y_final - 64.0).abs() < 1e-4,
            "DP45 cubic: got {}, expected 64.0",
            y_final
        );
    }

    #[test]
    fn dp45_adapts_to_stiff_region() {
        // dy/dx = -100*(y - sin(x)) + cos(x) - mildly stiff
        // Exact solution starting at y(0)=0 converges rapidly to sin(x)
        let solver = DormandPrince45::new(1e-8, 1e-8);
        let traj = solver.integrate(|x, y| -100.0 * (y - x.sin()) + x.cos(), 0.0, 0.0, 3.0);

        let (x_final, y_final) = traj.last().unwrap();
        let exact = x_final.sin();
        assert!(
            (y_final - exact).abs() < 1e-3,
            "DP45 stiff: got {}, expected {}",
            y_final,
            exact
        );
        // Adaptive solver should use more steps in the initial transient
        assert!(
            traj.len() > 10,
            "Should take multiple adaptive steps, got {}",
            traj.len()
        );
    }

    #[test]
    fn dp45_logistic_equation() {
        // dy/dx = y(1-y), y(0) = 0.01
        // Exact: y = 1/(1 + 99*exp(-x))
        let solver = DormandPrince45::new(1e-10, 1e-10);
        let traj = solver.integrate(|_x, y| y * (1.0 - y), 0.0, 0.01, 15.0);

        let (x_final, y_final) = traj.last().unwrap();
        let exact = 1.0 / (1.0 + 99.0 * (-x_final).exp());
        assert!(
            (y_final - exact).abs() < 1e-6,
            "DP45 logistic: got {}, expected {}",
            y_final,
            exact
        );
    }

    // ── Vector ODE Tests ─────────────────────────────────────────────────

    #[test]
    fn rk4_vector_harmonic_oscillator() {
        // dx/dt = v, dv/dt = -x  (simple harmonic oscillator)
        // y = [x, v], x(0) = 1, v(0) = 0
        // Exact: x = cos(t), v = -sin(t)
        let solver = RungeKutta4::new(0.001);
        let traj = solver.integrate_system(
            |_t, y| vec![y[1], -y[0]],
            0.0,
            &[1.0, 0.0],
            2.0 * std::f64::consts::PI, // full period
        );

        let (t_final, y_final) = traj.last().unwrap();
        assert!((t_final - 2.0 * std::f64::consts::PI).abs() < 1e-8);
        // After one full period, should return to (1, 0)
        assert!(
            (y_final[0] - 1.0).abs() < 1e-4,
            "SHO position: got {}, expected 1.0",
            y_final[0]
        );
        assert!(
            y_final[1].abs() < 1e-4,
            "SHO velocity: got {}, expected 0.0",
            y_final[1]
        );
    }

    // ── Convenience Function Tests ───────────────────────────────────────

    #[test]
    fn convenience_rk4_works() {
        let traj = solve_ode_rk4(|_x, y| -y, 0.0, 1.0, 2.0, 0.01);
        let (_, y_final) = traj.last().unwrap();
        let exact = (-2.0_f64).exp();
        assert!((y_final - exact).abs() < 1e-6);
    }

    #[test]
    fn convenience_adaptive_works() {
        let traj = solve_ode_adaptive(|_x, y| -y, 0.0, 1.0, 2.0, 1e-8, 1e-8);
        let (_, y_final) = traj.last().unwrap();
        let exact = (-2.0_f64).exp();
        assert!((y_final - exact).abs() < 1e-6);
    }

    #[test]
    fn rk4_trajectory_length() {
        let solver = RungeKutta4::new(0.5);
        let traj = solver.integrate(|_x, y| -y, 0.0, 1.0, 5.0);
        // 5.0 / 0.5 = 10 steps + initial point = 11 points
        assert_eq!(traj.len(), 11);
    }

    #[test]
    fn dp45_handles_zero_rhs() {
        // dy/dx = 0, y(0) = 42  =>  y = 42 everywhere
        let solver = DormandPrince45::new(1e-10, 1e-10);
        let traj = solver.integrate(|_x, _y| 0.0, 0.0, 42.0, 100.0);

        let (_, y_final) = traj.last().unwrap();
        assert!(
            (y_final - 42.0).abs() < 1e-10,
            "Constant: got {}, expected 42.0",
            y_final
        );
    }
}
