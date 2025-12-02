---
id: velvet_benchmarks
title: Benchmarks
sidebar_position: 8
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Benchmarks

Velvet includes performance benchmarks to measure and optimize critical operations.

## Overview

Benchmarks help identify performance bottlenecks and track performance improvements over time. They use the [Criterion](https://github.com/bheisler/criterion.rs) benchmarking framework.

## Location

Benchmarks are located in: `/packages/velvet/benches/`

## Available Benchmarks

### PDF Generation

**File**: `pdf_generation.rs`

Benchmarks the performance of generating PDF reports from tally results. This is a critical operation as report generation can be time-consuming for large elections.

Measures:
- PDF rendering time
- Memory usage
- Throughput for various document sizes

## Running Benchmarks

### All Benchmarks

Run all benchmarks:

```bash
cargo bench
```

### Specific Benchmark

Run a specific benchmark:

```bash
cargo bench --bench pdf_generation
```

## Understanding Results

Criterion provides detailed statistics:

- **Mean time** - Average execution time
- **Standard deviation** - Variability in measurements
- **Median** - Middle value of all measurements
- **Throughput** - Operations per second (when applicable)

Example output:

```
pdf_generation/small_report
                        time:   [45.234 ms 45.789 ms 46.345 ms]
                        thrpt:  [21.582 ops/s 21.847 ops/s 22.112 ops/s]
```

## Benchmark Reports

Criterion generates HTML reports in:

```
target/criterion/
```

Open `index.html` to see:
- Detailed statistics
- Performance graphs
- Comparisons with previous runs

## Writing Benchmarks

### Basic Example

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_function(c: &mut Criterion) {
    c.bench_function("operation_name", |b| {
        b.iter(|| {
            // Code to benchmark
            expensive_operation(black_box(input_data))
        });
    });
}

criterion_group!(benches, benchmark_function);
criterion_main!(benches);
```

### Parameterized Benchmarks

Test performance across different input sizes:

```rust
fn benchmark_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                b.iter(|| process_data(black_box(size)));
            },
        );
    }
    
    group.finish();
}
```

## Performance Targets

While specific targets depend on hardware, general goals:

- **Small elections** (< 1,000 votes) - Tally in < 1 second
- **Medium elections** (1,000-10,000 votes) - Tally in < 5 seconds
- **Large elections** (10,000-100,000 votes) - Tally in < 30 seconds
- **PDF generation** - < 100ms per page

## Optimization Tips

When optimizing Velvet:

1. **Profile first** - Identify actual bottlenecks before optimizing
2. **Benchmark changes** - Measure impact of optimizations
3. **Consider trade-offs** - Balance speed vs. code clarity
4. **Avoid premature optimization** - Focus on correctness first

## Configuration

Benchmark configuration in `Cargo.toml`:

```toml
[[bench]]
name = "pdf_generation"
harness = false
```

The `harness = false` setting tells Cargo to use Criterion's benchmark harness instead of the default.

## Continuous Benchmarking

Consider running benchmarks:

- Before merging performance-related changes
- Periodically to track performance trends
- When optimizing critical paths

*Further documentation to be added.*
