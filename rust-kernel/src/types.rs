use std::ffi::CString;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PcvRow {
    pub mutation_index: i32,
    pub sample_index: i32,
    pub ref_counts: i32,
    pub alt_counts: i32,
    pub major_cn: i32,
    pub minor_cn: i32,
    pub normal_cn: i32,
    pub tumour_content: f64,
    pub error_rate: f64,
}

#[repr(C)]
pub struct PcvConfig {
    pub num_clusters: i32,
    pub num_grid_points: i32,
    pub num_restarts: i32,
    pub max_iters: i32,
    pub print_freq: i32,
    pub kernel_threads: i32,
    pub convergence_threshold: f64,
    pub mix_weight_prior: f64,
    pub precision: f64,
    pub density: u8,
    pub use_seed: u8,
    pub seed: u64,
}

pub struct PcvResult {
    pub num_mutations: usize,
    pub num_samples: usize,
    pub num_clusters: usize,
    pub mutation_cluster_ids: Vec<i32>,
    pub mutation_cluster_probs: Vec<f64>,
    pub cluster_sample_prevalence: Vec<f64>,
    pub cluster_sample_prevalence_std: Vec<f64>,
}

pub struct PcvError {
    pub message: CString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    Binomial,
    BetaBinomial,
}

impl TryFrom<u8> for Density {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Binomial),
            1 => Ok(Self::BetaBinomial),
            _ => Err(format!("unknown density code: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MajorCnPrior {
    pub cn: Vec<[i32; 3]>,
    pub mu: Vec<[f64; 3]>,
    pub log_pi: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleDataPoint {
    pub a: i32,
    pub b: i32,
    pub cn: Vec<[i32; 3]>,
    pub mu: Vec<[f64; 3]>,
    pub log_pi: Vec<f64>,
    pub t: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogLikelihoodTensor {
    pub num_mutations: usize,
    pub num_samples: usize,
    pub num_grid_points: usize,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataPreprocessor {
    pub theta_update_data: Vec<f64>,
    pub z_update_data: Vec<f64>,
    pub theta_update_shape: (usize, usize),
    pub z_update_shape: usize,
    pub use_parallel: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Priors {
    pub pi: Vec<f64>,
    pub theta: Vec<f64>,
    pub log_theta: Vec<f64>,
    pub pi_log_gamma: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariationalParameters {
    pub pi: Vec<f64>,
    pub theta: Vec<f64>,
    pub z: Vec<f64>,
    pub num_clusters: usize,
    pub num_data_points: usize,
    pub num_dims: usize,
    pub num_grid_points: usize,
}
