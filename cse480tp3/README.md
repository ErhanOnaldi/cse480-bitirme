# CSE480 Term Project 3 (Phase 3)

This folder contains the Phase 3 implementation for the **1D Bin Packing Problem** metaheuristic.

## Quick start (Rust)

Run the example instance from TP2 (7 items, capacity 60):

```bash
cargo run --release -- run-example
```

Run an experiment table (each instance is run 5 times):

```bash
cargo run --release -- run-batch --runs 5
```

## Output

- The CLI prints the best solution found and a summary table (mean/best/std objective; mean/best time).
