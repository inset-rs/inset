//! Flutter counterpart: `gestures/lsq_solver.dart`.

use reveal_foundation::PRECISION_ERROR_TOLERANCE;

struct Vector {
    offset: usize,
    length: usize,
    elements: Vec<f64>,
}

impl Vector {
    fn new(size: usize) -> Vector {
        Vector {
            offset: 0,
            length: size,
            elements: vec![0.0; size],
        }
    }

    fn from_vol(values: Vec<f64>, offset: usize, length: usize) -> Vector {
        Vector {
            offset,
            length,
            elements: values,
        }
    }

    fn get(&self, i: usize) -> f64 {
        self.elements[i + self.offset]
    }

    fn set(&mut self, i: usize, value: f64) {
        self.elements[i + self.offset] = value;
    }

    fn dot(&self, a: &Vector) -> f64 {
        let mut result = 0.0;
        for i in 0..self.length {
            result += self.get(i) * a.get(i);
        }
        result
    }

    fn norm(&self) -> f64 {
        self.dot(self).sqrt()
    }
}

struct Matrix {
    columns: usize,
    elements: Vec<f64>,
}

impl Matrix {
    fn new(rows: usize, cols: usize) -> Matrix {
        Matrix {
            columns: cols,
            elements: vec![0.0; rows * cols],
        }
    }

    fn get(&self, row: usize, col: usize) -> f64 {
        self.elements[row * self.columns + col]
    }

    fn set(&mut self, row: usize, col: usize, value: f64) {
        self.elements[row * self.columns + col] = value;
    }

    fn get_row(&self, row: usize) -> Vector {
        let start = row * self.columns;
        let end = start + self.columns;
        Vector::from_vol(self.elements[start..end].to_vec(), 0, self.columns)
    }
}

/// An nth degree polynomial fit to a dataset.
pub struct PolynomialFit {
    /// The polynomial coefficients of the fit.
    ///
    /// For each `i`, the element `coefficients[i]` is the coefficient of
    /// the `i`-th power of the variable.
    pub coefficients: Vec<f64>,

    /// An indicator of the quality of the fit.
    ///
    /// Larger values indicate greater quality. The value ranges from 0.0 to 1.0.
    ///
    /// The confidence is defined as the fraction of the dataset's variance
    /// that is captured by variance in the fit polynomial. In statistics
    /// textbooks this is often called "r-squared".
    pub confidence: f64,
}

impl PolynomialFit {
    /// Creates a polynomial fit of the given degree.
    ///
    /// There are n + 1 coefficients in a fit of degree n.
    pub fn new(degree: usize) -> PolynomialFit {
        PolynomialFit {
            coefficients: vec![0.0; degree + 1],
            confidence: 0.0,
        }
    }
}

impl std::fmt::Display for PolynomialFit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let coefficient_string: Vec<String> = self
            .coefficients
            .iter()
            .map(|c| format!("{c:.3}"))
            .collect();
        write!(
            f,
            "PolynomialFit({coefficient_string:?}, confidence: {:.3})",
            self.confidence
        )
    }
}

/// Uses the least-squares algorithm to fit a polynomial to a set of data.
pub struct LeastSquaresSolver {
    /// The x-coordinates of each data point.
    pub x: Vec<f64>,

    /// The y-coordinates of each data point.
    pub y: Vec<f64>,

    /// The weight to use for each data point.
    pub w: Vec<f64>,
}

impl LeastSquaresSolver {
    /// Creates a least-squares solver.
    pub fn new(x: Vec<f64>, y: Vec<f64>, w: Vec<f64>) -> LeastSquaresSolver {
        debug_assert_eq!(x.len(), y.len());
        debug_assert_eq!(y.len(), w.len());
        LeastSquaresSolver { x, y, w }
    }

    /// Fits a polynomial of the given degree to the data points.
    ///
    /// When there is not enough data to fit a curve [`None`] is returned.
    pub fn solve(&self, degree: usize) -> Option<PolynomialFit> {
        if degree > self.x.len() {
            return None;
        }

        let mut result = PolynomialFit::new(degree);

        let m = self.x.len();
        let n = degree + 1;

        let mut a = Matrix::new(n, m);
        for h in 0..m {
            a.set(0, h, self.w[h]);
            for i in 1..n {
                a.set(i, h, a.get(i - 1, h) * self.x[h]);
            }
        }

        let mut q = Matrix::new(n, m);
        let mut r = Matrix::new(n, n);
        for j in 0..n {
            for h in 0..m {
                q.set(j, h, a.get(j, h));
            }
            for i in 0..j {
                let dot = q.get_row(j).dot(&q.get_row(i));
                for h in 0..m {
                    q.set(j, h, q.get(j, h) - dot * q.get(i, h));
                }
            }

            let norm = q.get_row(j).norm();
            if norm < PRECISION_ERROR_TOLERANCE {
                return None;
            }

            let inverse_norm = 1.0 / norm;
            for h in 0..m {
                q.set(j, h, q.get(j, h) * inverse_norm);
            }
            for i in 0..n {
                r.set(
                    j,
                    i,
                    if i < j {
                        0.0
                    } else {
                        q.get_row(j).dot(&a.get_row(i))
                    },
                );
            }
        }

        let mut wy = Vector::new(m);
        for h in 0..m {
            wy.set(h, self.y[h] * self.w[h]);
        }
        for i in (0..n).rev() {
            result.coefficients[i] = q.get_row(i).dot(&wy);
            for j in ((i + 1)..n).rev() {
                result.coefficients[i] -= r.get(i, j) * result.coefficients[j];
            }
            result.coefficients[i] /= r.get(i, i);
        }

        let mut y_mean = 0.0;
        for h in 0..m {
            y_mean += self.y[h];
        }
        y_mean /= m as f64;

        let mut sum_squared_error = 0.0;
        let mut sum_squared_total = 0.0;
        for h in 0..m {
            let mut term = 1.0;
            let mut err = self.y[h] - result.coefficients[0];
            for i in 1..n {
                term *= self.x[h];
                err -= term * result.coefficients[i];
            }
            sum_squared_error += self.w[h] * self.w[h] * err * err;
            let v = self.y[h] - y_mean;
            sum_squared_total += self.w[h] * self.w[h] * v * v;
        }

        result.confidence = if sum_squared_total <= PRECISION_ERROR_TOLERANCE {
            1.0
        } else {
            1.0 - (sum_squared_error / sum_squared_total)
        };

        Some(result)
    }
}
