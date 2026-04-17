import json,sys,numpy as np

# Purpose:
# This helper is embedded and executed by tyclone fit-mcmc --python-compatible to create
# PyClone-compatible MCMC initial state values from a single seeded legacy NumPy RNG.
# It is a stdin->stdout JSON filter: read seed/shape/hyperparameter config,
# emit cluster assignments and per-atom phi draws using the same Beta sampler
# that upstream PyClone MCMC uses (numpy.random.beta via the legacy global RNG).

# stdin JSON: {seed, num_mutations, num_samples, init_method,
#              base_measure_alpha, base_measure_beta, precision, alpha}
c=json.load(sys.stdin)
np.random.seed(int(c["seed"]))
N,D=int(c["num_mutations"]),int(c["num_samples"])
bma,bmb=float(c["base_measure_alpha"]),float(c["base_measure_beta"])
prec=float(c["precision"])
alpha=float(c["alpha"])

if c["init_method"]=="disconnected":
    cluster_id=list(range(N))
    num_atoms=N
else:
    cluster_id=[0]*N
    num_atoms=1

# Sample phi for each atom: shape (num_atoms, D), using legacy RNG (np.random.beta).
atoms_phi=np.random.beta(bma,bmb,size=(num_atoms,D)).ravel().tolist()

# stdout JSON: {cluster_id, atoms_phi, num_atoms, alpha, precision}
json.dump({"cluster_id":cluster_id,"atoms_phi":atoms_phi,"num_atoms":num_atoms,
           "alpha":alpha,"precision":prec if prec>0.0 else 1000.0},
          sys.stdout,separators=(",",":"))
