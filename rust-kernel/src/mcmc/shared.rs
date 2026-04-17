use crate::likelihood::{log_pyclone_beta_binomial_pdf, log_pyclone_binomial_pdf};
use crate::types::{ClusterAtom, Density, DpState, SampleDataPoint};
use statrs::function::gamma::ln_gamma;

use super::rng::McmcRng;

pub const EPS: f64 = 1e-6;
pub const AUX_NEW_CLUSTERS: usize = 2;

pub fn clip_unit_interval(value: f64) -> f64 {
    value.clamp(EPS, 1.0 - EPS)
}

pub fn sample_log_weights(log_weights: &[f64], rng: &mut dyn McmcRng) -> Result<usize, String> {
    rng.categorical_from_log_weights(log_weights)
}

pub fn log_beta_density(value: f64, alpha: f64, beta: f64) -> f64 {
    let p = clip_unit_interval(value);
    let log_norm = ln_gamma(alpha + beta) - ln_gamma(alpha) - ln_gamma(beta);
    log_norm + (alpha - 1.0) * p.ln() + (beta - 1.0) * (1.0 - p).ln()
}

pub fn base_measure_log_p_atom(atom: &ClusterAtom, alpha: f64, beta: f64) -> f64 {
    atom.phi
        .iter()
        .map(|&value| log_beta_density(value, alpha, beta))
        .sum()
}

pub fn mutation_log_likelihood(
    mutation_index: usize,
    atom: &ClusterAtom,
    data: &[SampleDataPoint],
    num_samples: usize,
    density: Density,
    precision: f64,
) -> f64 {
    (0..num_samples)
        .map(|sample_index| {
            let datum = &data[mutation_index * num_samples + sample_index];
            match density {
                Density::Binomial => log_pyclone_binomial_pdf(datum, atom.phi[sample_index]),
                Density::BetaBinomial => {
                    log_pyclone_beta_binomial_pdf(datum, atom.phi[sample_index], precision)
                }
            }
        })
        .sum()
}

pub fn cluster_members(state: &DpState) -> Vec<Vec<usize>> {
    let mut members = vec![Vec::new(); state.atoms.len()];
    for (mutation_index, &cluster_index) in state.cluster_id.iter().enumerate() {
        members[cluster_index].push(mutation_index);
    }
    members
}

pub fn cluster_atom_log_posterior(
    atom: &ClusterAtom,
    member_indices: &[usize],
    data: &[SampleDataPoint],
    num_samples: usize,
    density: Density,
    precision: f64,
    base_measure_alpha: f64,
    base_measure_beta: f64,
) -> f64 {
    let likelihood: f64 = member_indices
        .iter()
        .map(|&mutation_index| {
            mutation_log_likelihood(mutation_index, atom, data, num_samples, density, precision)
        })
        .sum();

    likelihood + base_measure_log_p_atom(atom, base_measure_alpha, base_measure_beta)
}

pub fn total_log_likelihood(
    state: &DpState,
    data: &[SampleDataPoint],
    num_samples: usize,
    density: Density,
) -> f64 {
    state
        .cluster_id
        .iter()
        .enumerate()
        .map(|(mutation_index, &cluster_index)| {
            mutation_log_likelihood(
                mutation_index,
                &state.atoms[cluster_index],
                data,
                num_samples,
                density,
                state.precision,
            )
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{base_measure_log_p_atom, log_beta_density};
    use crate::types::ClusterAtom;

    #[test]
    fn beta_log_density_prefers_center_for_symmetric_prior_above_one() {
        let center = log_beta_density(0.5, 2.0, 2.0);
        let edge = log_beta_density(0.1, 2.0, 2.0);
        assert!(center > edge);
    }

    #[test]
    fn cluster_base_measure_log_prob_sums_across_samples() {
        let centered = ClusterAtom {
            phi: vec![0.5, 0.5],
        };
        let edged = ClusterAtom {
            phi: vec![0.1, 0.9],
        };
        assert!(
            base_measure_log_p_atom(&centered, 2.0, 2.0)
                > base_measure_log_p_atom(&edged, 2.0, 2.0)
        );
    }
}
