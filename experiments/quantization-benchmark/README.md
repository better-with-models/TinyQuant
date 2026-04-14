# experiments/quantization-benchmark

Python benchmark suite that compares TinyQuant codec variants against common
storage baselines. It measures storage size, compression ratio, fidelity
(Pearson ρ, MSE, top-k recall), and compress/decompress throughput across nine
configurations: FP32, FP16, uint8 min-max, Product Quantization simulation,
and TinyQuant at 8-bit, 4-bit, and 2-bit with and without FP16 residuals.

## What lives here

| File | Role |
| --- | --- |
| `run_benchmark.py` | Main benchmark driver; imports `tinyquant_cpu.codec` and writes JSON results |
| `generate_embeddings.py` | Generates synthetic embedding fixtures into `data/` |
| `generate_plots.py` | Reads JSON results and writes charts into `results/` |
| `data/` | Generated fixture files (not committed) |
| `results/` | Generated JSON and plot output (not committed) |
| `REPORT.md` | Narrative analysis of a prior benchmark run |

## How this area fits the system

The scripts depend on the `tinyquant_cpu` Python package being installed in
the active environment (built from the Rust codec via PyO3/maturin). They are
run manually, not by CI. Outputs in `data/` and `results/` are local only.

## Common edit paths

- Changing codec configurations or adding a new baseline: `run_benchmark.py`
- Adjusting chart style or metrics displayed: `generate_plots.py`
- Changing the fixture distribution or dimensionality: `generate_embeddings.py`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
