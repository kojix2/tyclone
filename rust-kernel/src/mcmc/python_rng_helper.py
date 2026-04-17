import json
import os
import sys

import numpy as np


TRACE = os.environ.get("TYCLONE_PYTHON_TRACE") not in (None, "", "0")


def trace(message):
    if TRACE:
        sys.stderr.write(message + "\n")
        sys.stderr.flush()


def decode_state(raw_state):
    if raw_state is None:
        return None

    return (
        str(raw_state[0]),
        np.asarray(raw_state[1], dtype=np.uint32),
        int(raw_state[2]),
        int(raw_state[3]),
        None if raw_state[4] is None else float(raw_state[4]),
    )


def encode_state(state):
    return [
        state[0],
        state[1].tolist(),
        int(state[2]),
        int(state[3]),
        None if state[4] is None else float(state[4]),
    ]


def build_rng(req):
    raw_state = req.get("state")
    if raw_state is None:
        return np.random.RandomState(int(req["seed"]))

    rng = np.random.RandomState()
    rng.set_state(decode_state(raw_state))
    return rng


def main():
    req = json.load(sys.stdin)
    req_id = req.get("id")
    op = req.get("op", "unknown")
    trace("[python-rng-helper] recv id={} op={}".format(req_id, op))

    try:
        rng = build_rng(req)

        if op == "shuffle":
            if "values" in req:
                values = np.asarray(req["values"], dtype=np.int64)
                rng.shuffle(values)
                result = values.tolist()
            else:
                result = rng.permutation(int(req["n"])).tolist()
        elif op == "uniform":
            result = float(rng.random_sample())
        elif op == "beta_vec":
            result = rng.beta(
                float(req["alpha"]),
                float(req["beta"]),
                size=int(req["count"]),
            ).tolist()
        elif op == "gamma":
            result = float(rng.gamma(float(req["shape"]), 1.0 / float(req["rate"])))
        elif op == "categorical_from_log_weights":
            log_weights = np.asarray(req["log_weights"], dtype=float)
            if log_weights.size == 0:
                raise ValueError("log_weights must not be empty")
            max_log = float(np.max(log_weights))
            weights = np.exp(log_weights - max_log)
            norm = float(np.sum(weights))
            if not np.isfinite(norm) or norm <= 0.0:
                raise ValueError("all candidate log-weights are non-finite")

            threshold = float(rng.random_sample())
            probs = weights / norm
            selected = len(probs) - 1
            for idx, prob in enumerate(probs):
                threshold -= float(prob)
                if threshold <= 0.0:
                    selected = idx
                    break
            result = int(selected)
        else:
            raise ValueError("unknown op: {}".format(op))

        trace("[python-rng-helper] send id={} op={}".format(req_id, op))
        json.dump(
            {
                "id": req_id,
                "ok": True,
                "result": result,
                "state": encode_state(rng.get_state()),
            },
            sys.stdout,
            separators=(",", ":"),
        )
    except Exception as exc:
        trace("[python-rng-helper] error id={} op={} err={}".format(req_id, op, exc))
        json.dump(
            {
                "id": req_id,
                "ok": False,
                "error": "{}: {}".format(op, exc),
                "state": req.get("state"),
            },
            sys.stdout,
            separators=(",", ":"),
        )

    sys.stdout.write("\n")
    sys.stdout.flush()


main()