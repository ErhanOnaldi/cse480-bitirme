# Datasets

`cse480tp3` expects **1D Bin Packing** instances.

## Supported input format

### 1) Simple single-instance (recommended)

Each file must contain only integers (plus optional full-line `#` comments), in one of these exact layouts:

1) `n capacity size1 ... size_n`
2) `capacity n size1 ... size_n`

### 2) BinPack multi-instance files (your `binpack*.txt`)

We also support the common “BinPack” layout used by many benchmark packs:

- First line: `K` (number of instances in the file)
- Then repeated `K` times:
  - Instance name (string, e.g. `t501_00`)
  - Header: `<capacity> <n> <opt?>` (capacity/items, optional third number ignored)
  - `n` item sizes (one per line; can be integers or decimals like `36.6`)

Decimals are automatically scaled (e.g. `100.0` becomes `1000` and `36.6` becomes `366`).

## Run all instances in this directory

From `cse480tp3/`:

```bash
cargo run --release -- run-dir ../datasets --runs 5
```

Tip: If the directory contains many instances, limit runtime with:

```bash
cargo run --release -- run-dir ../datasets --runs 1 --take 10 --time-limit-s 0.2
```
