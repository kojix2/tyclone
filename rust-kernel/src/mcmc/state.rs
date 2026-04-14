use crate::preprocess::build_sample_data_point;
use crate::types::{DpState, PcvMcmcConfig, PcvRow, SampleDataPoint};
use rand::Rng;

use super::shared::{clip_unit_interval, sample_prior_atom, EPS};

pub fn build_data_matrix(
    rows: &[PcvRow],
    num_mutations: usize,
    num_samples: usize,
) -> Result<(Vec<SampleDataPoint>, Vec<f64>), String> {
    if rows.len() != num_mutations * num_samples {
        return Err("rows length must equal num_mutations * num_samples".to_string());
    }

    let mut data = vec![None; num_mutations * num_samples];
    let mut observed_phi = vec![0.5; num_mutations * num_samples];

    for row in rows {
        if row.mutation_index < 0 || row.sample_index < 0 {
            return Err("mutation_index and sample_index must be >= 0".to_string());
        }

        let mutation_index = row.mutation_index as usize;
        let sample_index = row.sample_index as usize;
        if mutation_index >= num_mutations || sample_index >= num_samples {
            return Err("row index out of bounds for tensor shape".to_string());
        }

        let offset = mutation_index * num_samples + sample_index;
        if data[offset].is_some() {
            return Err("duplicate mutation/sample pair encountered".to_string());
        }

        data[offset] = Some(build_sample_data_point(row)?);

        let depth = (row.ref_counts + row.alt_counts).max(1) as f64;
        let raw_vaf = row.alt_counts as f64 / depth;
        let phi = if row.tumour_content > 0.0 {
            raw_vaf / row.tumour_content.max(EPS)
        } else {
            0.5
        };
        observed_phi[offset] = clip_unit_interval(phi.min(1.0));
    }

    let collected = data
        .into_iter()
        .map(|value| value.ok_or_else(|| "missing mutation/sample pair encountered".to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((collected, observed_phi))
}

pub fn initialize_state(
    cfg: &PcvMcmcConfig,
    num_mutations: usize,
    num_samples: usize,
    _observed_phi: &[f64],
    rng: &mut impl Rng,
) -> Result<DpState, String> {
    let disconnected = cfg.init_method == 0;

    let cluster_id = if disconnected {
        (0..num_mutations).collect::<Vec<_>>()
    } else {
        vec![0usize; num_mutations]
    };

    let initial_clusters = if disconnected {
        num_mutations.max(1)
    } else {
        1
    };
    let mut atoms = Vec::with_capacity(initial_clusters);
    for _ in 0..initial_clusters {
        atoms.push(sample_prior_atom(
            num_samples,
            cfg.base_measure_alpha,
            cfg.base_measure_beta,
            rng,
        )?);
    }

    Ok(DpState {
        cluster_id,
        atoms,
        alpha: cfg.alpha,
        precision: if cfg.precision > 0.0 {
            cfg.precision
        } else {
            1000.0
        },
    })
}

#[cfg(test)]
mod tests {
    use super::initialize_state;
    use crate::types::PcvMcmcConfig;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn config(init_method: u8) -> PcvMcmcConfig {
        PcvMcmcConfig {
            num_iters: 10,
            burnin: 0,
            thin: 1,
            num_clusters: 8,
            alpha: 1.0,
            alpha_prior_shape: 1.0,
            alpha_prior_rate: 0.001,
            init_method,
            base_measure_alpha: 1.0,
            base_measure_beta: 1.0,
            mh_step_size: 0.01,
            mh_precision_step: 0.0,
            mh_precision_proposal_precision: 0.01,
            precision: 1000.0,
            density: 1,
            use_seed: 1,
            seed: 7,
            print_freq: 0,
        }
    }

    #[test]
    fn disconnected_initialization_allocates_each_mutation_to_its_own_cluster() {
        let mut rng = StdRng::seed_from_u64(7);
        let state = initialize_state(&config(0), 5, 3, &vec![0.5; 15], &mut rng).unwrap();
        assert_eq!(state.atoms.len(), 5);
        assert_eq!(state.cluster_id, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn connected_initialization_allocates_all_mutations_to_one_cluster() {
        let mut rng = StdRng::seed_from_u64(7);
        let state = initialize_state(&config(1), 5, 3, &vec![0.5; 15], &mut rng).unwrap();
        assert_eq!(state.atoms.len(), 1);
        assert_eq!(state.cluster_id, vec![0, 0, 0, 0, 0]);
    }
}
