use statrs::function::gamma::ln_gamma;

use crate::math::log_sum_exp;
use crate::types::{Density, SampleDataPoint};

fn log_beta(a: f64, b: f64) -> f64 {
    ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
}

fn log_binomial_coefficient(n: i32, x: i32) -> f64 {
    ln_gamma((n + 1) as f64) - ln_gamma((n - x + 1) as f64) - ln_gamma((x + 1) as f64)
}

pub fn log_beta_binomial_pdf(n: i32, x: i32, a: f64, b: f64) -> f64 {
    log_binomial_coefficient(n, x) + log_beta(a + x as f64, b + (n - x) as f64) - log_beta(a, b)
}

pub fn log_binomial_pdf(n: i32, x: i32, p: f64) -> f64 {
    log_binomial_coefficient(n, x) + (x as f64) * p.ln() + ((n - x) as f64) * (-p).ln_1p()
}

pub fn log_pyclone_binomial_pdf(data: &SampleDataPoint, f: f64) -> f64 {
    let t = data.t;
    let population_prior = [1.0 - t, t * (1.0 - f), t * f];
    let mut ll = vec![f64::NEG_INFINITY; data.cn.len()];

    for (c_idx, cn) in data.cn.iter().enumerate() {
        let mut expected_vaf = 0.0;
        let mut norm_const = 0.0;

        for i in 0..3 {
            let expected_cn = population_prior[i] * cn[i] as f64;
            expected_vaf += expected_cn * data.mu[c_idx][i];
            norm_const += expected_cn;
        }

        expected_vaf /= norm_const;
        ll[c_idx] = data.log_pi[c_idx] + log_binomial_pdf(data.a + data.b, data.b, expected_vaf);
    }

    log_sum_exp(&ll)
}

pub fn log_pyclone_beta_binomial_pdf(data: &SampleDataPoint, f: f64, precision: f64) -> f64 {
    let t = data.t;
    let population_prior = [1.0 - t, t * (1.0 - f), t * f];
    let mut ll = vec![f64::NEG_INFINITY; data.cn.len()];

    for (c_idx, cn) in data.cn.iter().enumerate() {
        let mut expected_vaf = 0.0;
        let mut norm_const = 0.0;

        for i in 0..3 {
            let expected_cn = population_prior[i] * cn[i] as f64;
            expected_vaf += expected_cn * data.mu[c_idx][i];
            norm_const += expected_cn;
        }

        expected_vaf /= norm_const;
        let alpha = expected_vaf * precision;
        let beta = precision - alpha;
        ll[c_idx] = data.log_pi[c_idx] + log_beta_binomial_pdf(data.a + data.b, data.b, alpha, beta);
    }

    log_sum_exp(&ll)
}

pub fn compute_likelihood_grid(
    data: &SampleDataPoint,
    ccf_grid: &[f64],
    density: Density,
    precision: f64,
) -> Result<Vec<f64>, String> {
    if ccf_grid.is_empty() {
        return Err("ccf_grid must not be empty".to_string());
    }
    if precision <= 0.0 {
        return Err("precision must be > 0".to_string());
    }

    let grid = ccf_grid
        .iter()
        .map(|&ccf| match density {
            Density::Binomial => log_pyclone_binomial_pdf(data, ccf),
            Density::BetaBinomial => log_pyclone_beta_binomial_pdf(data, ccf, precision),
        })
        .collect();

    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::{compute_likelihood_grid, log_beta_binomial_pdf, log_binomial_pdf};
    use crate::preprocess::{build_sample_data_point, get_ccf_grid};
    use crate::types::{Density, PcvRow};

    fn approx_eq(left: f64, right: f64, tol: f64) {
        let delta = (left - right).abs();
        assert!(delta < tol, "left={left}, right={right}, delta={delta}, tol={tol}");
    }

    #[test]
    fn binomial_pdf_matches_closed_form_half_probability() {
        let actual = log_binomial_pdf(10, 5, 0.5);
        let expected = (252.0_f64 / 1024.0).ln();
        approx_eq(actual, expected, 1e-12);
    }

    #[test]
    fn beta_binomial_pdf_is_finite_for_valid_inputs() {
        let actual = log_beta_binomial_pdf(10, 5, 20.0, 20.0);
        assert!(actual.is_finite());
    }

    #[test]
    fn computes_sample_likelihood_grid_for_binomial_density() {
        let row = PcvRow {
            mutation_index: 0,
            sample_index: 0,
            ref_counts: 10,
            alt_counts: 5,
            major_cn: 2,
            minor_cn: 1,
            normal_cn: 2,
            tumour_content: 1.0,
            error_rate: 1e-3,
        };

        let data = build_sample_data_point(&row).unwrap();
        let grid = get_ccf_grid(5, 1e-6).unwrap();
        let ll = compute_likelihood_grid(&data, &grid, Density::Binomial, 200.0).unwrap();

        assert_eq!(ll.len(), 5);
        assert!(ll.iter().all(|value| value.is_finite()));
        assert!(ll[0] < ll[1]);
        assert!(ll[1] < ll[2]);
        assert!(ll[2] < ll[3]);
        assert!(ll[3] < ll[4]);
    }

    #[test]
    fn computes_sample_likelihood_grid_for_beta_binomial_density() {
        let row = PcvRow {
            mutation_index: 0,
            sample_index: 0,
            ref_counts: 10,
            alt_counts: 5,
            major_cn: 2,
            minor_cn: 1,
            normal_cn: 2,
            tumour_content: 1.0,
            error_rate: 1e-3,
        };

        let data = build_sample_data_point(&row).unwrap();
        let grid = get_ccf_grid(5, 1e-6).unwrap();
        let ll = compute_likelihood_grid(&data, &grid, Density::BetaBinomial, 200.0).unwrap();

        assert_eq!(ll.len(), 5);
        assert!(ll.iter().all(|value| value.is_finite()));
    }
}
