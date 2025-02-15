# Elastic Hash Table in Rust

This repository implements an **Elastic Hash Table** in Rust based on techniques described in the research paper [*Optimal Bounds for Open Addressing Without Reordering*](https://arxiv.org/abs/2501.02305v1). The core elastic hashing functionality is complete, while a WebAssembly (WASM) demo is planned for the future.

## Overview

The elastic hash table is designed for open addressing without reordering and achieves optimal probe complexities in both average and worst-case scenarios. The design draws inspiration from the techniques introduced in the original paper by Farach-Colton, Krapivin, and Kuszmaul, which include:

- **Elastic Hashing:** A method that uses a multi-level table structure with geometric capacity reduction.
- **Quadratic Probing:** A probing strategy implemented with Rust's `DefaultHasher` to simulate random probe sequences.
- **Load-Dependent Probing:** Using a probe limit function _f(ε)_ = _c · min(log₂(1/ε), log₂(1/δ))_ to balance insertions between levels.

Currently, only the elastic hashing portion is implemented. Future work will include a web demo (using WebAssembly) and additional algorithms (e.g., funnel hashing).


## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/)

### Clone the Repository

```bash
git clone https://github.com/asheehuang/hashing-wasm.git
cd hashing-wasm
```

### Build and Run Tests

```bash
cargo build
cargo test
```

## Usage

Below is a simple example of how to use the elastic hash table in your Rust project:

```rust
use elastic_hash_table::ElasticHashTable;

fn main() {
    // Create an elastic hash table with a total capacity of 128 and a delta of 0.1.
    let mut table = ElasticHashTable::new(128, 0.1);

    // Insert some key-value pairs.
    for i in 0..50 {
        table.insert(i, format!("Value {}", i)).expect("Insertion failed");
    }

    // Search for keys and print their corresponding values.
    for i in 0..50 {
        if let Some(val) = table.search(&i) {
            println!("Key {}: {}", i, val);
        } else {
            println!("Key {} not found.", i);
        }
    }
}
```

## Reference

The design of this elastic hash table is based on the techniques described in:

- **Farach-Colton, Martín; Krapivin, Andrew; Kuszmaul, William.** *Optimal Bounds for Open Addressing Without Reordering.* [arXiv:2501.02305v1](https://arxiv.org/abs/2501.02305v1)

## Future Work

- **Web Demo:** Develop a WebAssembly (WASM) demo with an interactive UI to visualize the behavior of the elastic hash table.
- **Funnel Hashing:** Implement and integrate funnel hashing as described in the original paper.
- **Performance Benchmarks:** Provide detailed performance comparisons and benchmarks.

## Contributing

Contributions, issues, and feature requests are welcome! Please check the [issues page](https://github.com/asheehuang/hashing-wasm/issues) for details.

---

Feel free to adjust any details (such as repository URL, contributing guidelines, or future work) to better suit your project.
