# AGENTS.md — Guide for AI Agents Working in `scripts/calibration`

`scripts/calibration/` contains helper scripts for producing calibration inputs.

## Layout

```text
scripts/calibration/
├── gen_openai_sample.py
└── README.md
```

## Common Workflows

### Update the calibration helper

1. Keep argument handling and output format documented in `README.md`.
2. Preserve compatibility with the downstream Rust calibration workflow.
3. Re-run the script with `--help` and one representative invocation after
   changing it.

## Invariants — Do Not Violate

- Calibration helpers here produce inputs; they do not define benchmark policy.
- Python entrypoints in this directory keep module docstrings.
- If output shape or CLI flags change, update the downstream tests or workflow
  references in the same change.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [rust/crates/tinyquant-bench/AGENTS.md](../../rust/crates/tinyquant-bench/AGENTS.md)
