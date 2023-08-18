<!--
SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@nsequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# Benchmarks

We use [criterion] for benchmarks. You can run the `service.rs` benchmarks with:

```bash
cargo bench --all-features --bench service -- --plotting-backend gnuplot
```

This could have a similar output to:

```
[.. compiler output ..]
    Finished bench [optimized] target(s) in 1.13s
     Running benches/service.rs (target/release/deps/service-3ab83f147e50c0df)
service/add_entry       time:   [653.06 µs 667.00 µs 682.25 µs]
                        thrpt:  [1.4657 Kelem/s 1.4993 Kelem/s 1.5313 Kelem/s]
                 change:
                        time:   [-23.873% -9.8609% +0.9240%] (p = 0.23 > 0.05)
                        thrpt:  [-0.9155% +10.940% +31.359%]
                        No change in performance detected.
Found 8 outliers among 100 measurements (8.00%)
  4 (4.00%) high mild
  4 (4.00%) high severe
```

This will not only output the benchmark results in the command line output, but
also generate an html report in `target/criterion/report/index.html`.

Note that we are enabling all features when running the benchmark - for example
the service requires the `build-server` feature, otherwise build would fail.

[criterion]: https://crates.io/crates/criterion