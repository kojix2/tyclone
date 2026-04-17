use crate::math::log_sum_exp;
use crate::types::ClusterAtom;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};
use rand_distr::{Beta, Distribution, Gamma};
use serde_json::{json, Value};
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

const EPS: f64 = 1e-6;
const PYTHON_RNG_HELPER_CODE: &str = include_str!("python_rng_helper.py");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PythonRngMode {
    Off,
    Online,
}

impl TryFrom<u8> for PythonRngMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Off),
            1 => Ok(Self::Online),
            _ => Err(format!("invalid python_rng_mode: {value}")),
        }
    }
}

pub trait McmcRng {
    fn uniform_f64(&mut self) -> Result<f64, String>;
    fn sample_prior_atom(
        &mut self,
        num_samples: usize,
        alpha: f64,
        beta: f64,
    ) -> Result<ClusterAtom, String>;
    fn shuffle_indices(&mut self, values: &mut [usize]) -> Result<(), String>;
    fn gamma_sample(&mut self, shape: f64, rate: f64) -> Result<f64, String>;

    fn sample_prior_atoms(
        &mut self,
        count: usize,
        num_samples: usize,
        alpha: f64,
        beta: f64,
    ) -> Result<Vec<ClusterAtom>, String> {
        (0..count)
            .map(|_| self.sample_prior_atom(num_samples, alpha, beta))
            .collect()
    }

    fn categorical_from_log_weights(&mut self, log_weights: &[f64]) -> Result<usize, String> {
        if log_weights.is_empty() {
            return Err("log_weights must not be empty".to_string());
        }
        let norm = log_sum_exp(log_weights);
        if !norm.is_finite() {
            return Err("all candidate log-weights are non-finite".to_string());
        }

        let mut threshold = self.uniform_f64()?;
        for (index, &log_weight) in log_weights.iter().enumerate() {
            threshold -= (log_weight - norm).exp();
            if threshold <= 0.0 {
                return Ok(index);
            }
        }

        Ok(log_weights.len() - 1)
    }
}

pub struct LocalMcmcRng {
    inner: StdRng,
}

impl LocalMcmcRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: StdRng::seed_from_u64(seed),
        }
    }
}

impl McmcRng for LocalMcmcRng {
    fn uniform_f64(&mut self) -> Result<f64, String> {
        Ok(self.inner.random::<f64>())
    }

    fn sample_prior_atom(
        &mut self,
        num_samples: usize,
        alpha: f64,
        beta: f64,
    ) -> Result<ClusterAtom, String> {
        let beta_dist = Beta::new(alpha, beta)
            .map_err(|error| format!("failed to initialize beta base measure: {error}"))?;
        let phi = (0..num_samples)
            .map(|_| beta_dist.sample(&mut self.inner).clamp(EPS, 1.0 - EPS))
            .collect();
        Ok(ClusterAtom { phi })
    }

    fn shuffle_indices(&mut self, values: &mut [usize]) -> Result<(), String> {
        values.shuffle(&mut self.inner);
        Ok(())
    }

    fn gamma_sample(&mut self, shape: f64, rate: f64) -> Result<f64, String> {
        let gamma = Gamma::new(shape, 1.0 / rate)
            .map_err(|error| format!("failed to initialize gamma sampler: {error}"))?;
        Ok(gamma.sample(&mut self.inner).max(EPS))
    }
}

pub struct PythonMcmcRng {
    python_executable: String,
    seed: u64,
    state: Option<Value>,
    next_id: u64,
}

impl PythonMcmcRng {
    pub fn new(seed: u64) -> Result<Self, String> {
        Ok(Self {
            python_executable: resolve_python_executable(),
            seed,
            state: None,
            next_id: 1,
        })
    }

    fn request(&mut self, op: &str, payload: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let trace_enabled = python_trace_enabled();

        let request = json!({
            "id": id,
            "op": op,
            "seed": self.seed,
            "state": self.state,
            "payload": payload
        });
        let mut request_object = request.as_object().cloned().ok_or_else(|| {
            format!("--python-compatible=online: failed to build request during {op}")
        })?;
        if let Some(payload_object) = request_object.remove("payload") {
            if let Some(payload_map) = payload_object.as_object() {
                for (key, value) in payload_map {
                    request_object.insert(key.clone(), value.clone());
                }
            }
        }

        if trace_enabled {
            eprintln!("[tyclone:python-rng] -> id={} op={} stage=spawn", id, op);
        }

        let encoded =
            serde_json::to_vec(&Value::Object(request_object.clone())).map_err(|error| {
                format!("--python-compatible=online: failed to encode request during {op}: {error}")
            })?;

        let mut child = Command::new(&self.python_executable)
            .arg("-c")
            .arg(PYTHON_RNG_HELPER_CODE)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                format!(
                    "--python-compatible=online: failed to start python helper with {} during {}: {}",
                    self.python_executable, op, error
                )
            })?;

        if trace_enabled {
            eprintln!(
                "[tyclone:python-rng] -> id={} op={} stage=write bytes={}",
                id,
                op,
                encoded.len()
            );
        }

        {
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                format!("--python-compatible=online: failed to capture helper stdin during {op}")
            })?;
            stdin.write_all(&encoded).map_err(|error| {
                format!("--python-compatible=online: failed to write request during {op}: {error}")
            })?;
        }
        let _ = child.stdin.take();

        if trace_enabled {
            eprintln!("[tyclone:python-rng] -> id={} op={} stage=wait", id, op);
        }

        let output = child.wait_with_output().map_err(|error| {
            format!("--python-compatible=online: failed to wait for helper during {op}: {error}")
        })?;
        if !output.status.success() {
            return Err(format!(
                "--python-compatible=online: python helper exited with code {:?} during {}",
                output.status.code(),
                op
            ));
        }

        if output.stdout.is_empty() {
            return Err(format!(
                "--python-compatible=online: helper produced no output during {op}"
            ));
        }

        if trace_enabled {
            eprintln!(
                "[tyclone:python-rng] <- id={} op={} bytes={}",
                id,
                op,
                output.stdout.len()
            );
        }

        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!("--python-compatible=online: invalid JSON response during {op}: {error}")
        })?;
        let response_id = response.get("id").and_then(Value::as_u64).ok_or_else(|| {
            format!("--python-compatible=online: missing response id during {op}")
        })?;
        if response_id != id {
            return Err(format!(
                "--python-compatible=online: response id mismatch during {op}: got {response_id}, expected {id}"
            ));
        }
        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            let message = response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown sidecar error");
            return Err(format!(
                "--python-compatible=online: sidecar returned error during {op}: {message}"
            ));
        }

        self.state = Some(
            response
                .get("state")
                .cloned()
                .ok_or_else(|| format!("--python-compatible=online: missing state during {op}"))?,
        );

        response
            .get("result")
            .cloned()
            .ok_or_else(|| format!("--python-compatible=online: missing result during {op}"))
    }
}

fn resolve_python_executable() -> String {
    env::var("TYCLONE_PYTHON")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "python3".to_string())
}

fn python_trace_enabled() -> bool {
    env::var("TYCLONE_PYTHON_TRACE")
        .ok()
        .map(|value| value != "0" && !value.is_empty())
        .unwrap_or(false)
}

impl McmcRng for PythonMcmcRng {
    fn uniform_f64(&mut self) -> Result<f64, String> {
        self.request("uniform", json!({}))?
            .as_f64()
            .ok_or_else(|| "--python-compatible=online: uniform returned non-float".to_string())
    }

    fn sample_prior_atom(
        &mut self,
        num_samples: usize,
        alpha: f64,
        beta: f64,
    ) -> Result<ClusterAtom, String> {
        let values = self.request(
            "beta_vec",
            json!({"count": num_samples, "alpha": alpha, "beta": beta}),
        )?;
        let raw = values
            .as_array()
            .ok_or_else(|| "--python-compatible=online: beta_vec returned non-array".to_string())?;
        if raw.len() != num_samples {
            return Err(format!(
                "--python-compatible=online: beta_vec length mismatch: got {}, expected {}",
                raw.len(),
                num_samples
            ));
        }
        let phi = raw
            .iter()
            .map(|value| {
                value
                    .as_f64()
                    .map(|v| v.clamp(EPS, 1.0 - EPS))
                    .ok_or_else(|| {
                        "--python-compatible=online: beta_vec returned non-float".to_string()
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ClusterAtom { phi })
    }

    fn sample_prior_atoms(
        &mut self,
        count: usize,
        num_samples: usize,
        alpha: f64,
        beta: f64,
    ) -> Result<Vec<ClusterAtom>, String> {
        let total_count = count.checked_mul(num_samples).ok_or_else(|| {
            "--python-compatible=online: beta_vec request size overflow".to_string()
        })?;
        let values = self.request(
            "beta_vec",
            json!({"count": total_count, "alpha": alpha, "beta": beta}),
        )?;
        let raw = values
            .as_array()
            .ok_or_else(|| "--python-compatible=online: beta_vec returned non-array".to_string())?;
        if raw.len() != total_count {
            return Err(format!(
                "--python-compatible=online: beta_vec length mismatch: got {}, expected {}",
                raw.len(),
                total_count
            ));
        }

        let flattened = raw
            .iter()
            .map(|value| {
                value
                    .as_f64()
                    .map(|v| v.clamp(EPS, 1.0 - EPS))
                    .ok_or_else(|| {
                        "--python-compatible=online: beta_vec returned non-float".to_string()
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(flattened
            .chunks(num_samples)
            .map(|chunk| ClusterAtom {
                phi: chunk.to_vec(),
            })
            .collect())
    }

    fn shuffle_indices(&mut self, values: &mut [usize]) -> Result<(), String> {
        let shuffled = self.request("shuffle", json!({"values": values}))?;
        let raw = shuffled
            .as_array()
            .ok_or_else(|| "--python-compatible=online: shuffle returned non-array".to_string())?;
        if raw.len() != values.len() {
            return Err(format!(
                "--python-compatible=online: shuffle length mismatch: got {}, expected {}",
                raw.len(),
                values.len()
            ));
        }
        for (slot, value) in values.iter_mut().zip(raw.iter()) {
            let parsed = value.as_u64().ok_or_else(|| {
                "--python-compatible=online: shuffle returned non-integer".to_string()
            })?;
            *slot = parsed as usize;
        }
        Ok(())
    }

    fn gamma_sample(&mut self, shape: f64, rate: f64) -> Result<f64, String> {
        self.request("gamma", json!({"shape": shape, "rate": rate}))?
            .as_f64()
            .map(|value| value.max(EPS))
            .ok_or_else(|| "--python-compatible=online: gamma returned non-float".to_string())
    }

    fn categorical_from_log_weights(&mut self, log_weights: &[f64]) -> Result<usize, String> {
        self.request(
            "categorical_from_log_weights",
            json!({"log_weights": log_weights}),
        )?
        .as_u64()
        .map(|value| value as usize)
        .ok_or_else(|| {
            "--python-compatible=online: categorical_from_log_weights returned non-integer"
                .to_string()
        })
    }
}
