# packaging

Packaging verification tests for the Python fat wheel and selector logic.

## Files

| File | Purpose |
| --- | --- |
| `test_assemble_fat_wheel.py` | Validate the assembled wheel layout and copied files |
| `test_python_fatwheel_workflow.py` | Exercise the Phase 24 fat-wheel workflow |
| `test_selector_detection.py` | Verify platform selector detection behavior |
| `test_shim_parity.py` | Check the Python shim stays aligned with the packaged surface |

## Running

```bash
pytest tests/packaging -q
```

## See Also

- [Local AGENTS.md](./AGENTS.md)
- [Parent README](../README.md)
- [scripts/packaging/README.md](../../scripts/packaging/README.md)
