<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
**Velvet** -- A type of woven tufted fabric, implying smoothness and luxury.

# Design

`velvet` will be a _cargo crate_. It will produce a _binary_ and a _lib_.

The _binary_ can be invoked as a _CLI_ program. The _lib_ can be used in another endpoint such as _harvest_.

## Binary

You can call velvet from the command line like this:

```
$> velvet run {stage} {optional-pipe} \
  --config ./path/to/velvet-config.json \
  --input-dir ./path/to/input-dir \
  --output-dir ./path/to/output-dir
```

### File and directory structure

##### velvet config

Example of a configuration file `--config`: `velvet-config.json`:

```json
{
  "version": "0.0.0",
  "stages": {
    "order": ["main"],
    "main": {
      "pipeline": [
        {
          "id": "decode-ballots",
          "pipe": "VelvetDecodeBallots",
          "config": {}
        },
        {
          "id": "do-tally",
          "pipe": "VelvetDoTally",
          "config": {
            "invalidateVotes": "Fail"
          }
        },
        {
          "id": "consolidation",
          "pipe": "VelvetConsolidation",
          "config": {}
        },
        {
          "id": "ties-resolution",
          "pipe": "VelvetTiesResolution",
          "config": {}
        },
        {
          "id": "compute-result",
          "pipe": "VelvetComputeResult",
          "config": {}
        },
        {
          "id": "gen-report",
          "pipe": "VelvetGenerateReport",
          "config": {
            "formats": ["pdf", "csv"]
          }
        }
      ]
    }
  }
}
```

### Input dir

Input directory contains multiples input directories:

```
./path/to/input-dir/default/
./path/to/input-dir/extra1/
./path/to/input-dir/extra2/
./path/to/input-dir/other/

```

The default input directory is mandatory and the other additionnal input directories should be set in the `velvet-config.json` file.

Configs are split into this file structure:

```
./path/to/input-dir/default/configs/
|-- election__<uuid>/
    |-- election-config.json
    |-- contest__<uuid>/
        |-- contest-config.json
        |-- area__<uuid>/
            |-- area-config.json
```

Ballots are split into this file structure:

```
./path/to/input-dir/default/ballots/
|-- election__<uuid>/
	|-- contest__<uuid>/
		|-- area__<uuid>/
			|-- ballots.csv
```

Same thing applies for `inputExtraDir`.

Therefore, the _entities_ are defined with:

- elections
- contests
- areas for contests
- according ballots

`ballots__<uuid>.csv` format, typically new-line separator value file:

```
<encoded-ballot-integer-1>
<encoded-ballot-integer-2>
<encoded-ballot-integer-3>
```

### Output dir

#### Processed pipe

Storing the stages in `./path/to/output-dir/status.json`

```json
{
  "lastExecutedPipe": "main.do-tally",
  "status": "Completed"
}
```

`lastExecutedPipe` should be formated as: `<stage>.<pipe-id>`

`status`: "Completed", "Error", "Interupted", ...

There will be as many output dir as many pipes, thus they will look like `./path/to/output-dir/<stage>/<pipe-id>/`:

```
./path/to/output-dir/main/decode-ballots/
./path/to/output-dir/main/do-tally/
./path/to/output-dir/main/consolidation/
./path/to/output-dir/main/ties-resolution/
./path/to/output-dir/main/compute-result/
./path/to/output-dir/main/generate-report/
```

For example, the _VelvetDecodeBallots_ output dir will look like that:

```
./path/to/output-dir/main/decode-ballots/
|-- election__<uuid>/
	|-- contest__<uuid>/
		|-- area__<uuid>/
			|-- ballot__<uuid>.csv # this is decoded
|-- output.log
```

Then the _VelvetDoTally_ output dir:

```
./path/to/output-dir/main/do-tally
|-- result.json
|-- output.log
```

The _VelvetConsolidation_ will fetch all `result.json` as input to process.

# Implementation

## Pipes

There will be a number of _pipes_. They could be:

- DecodeBallots
- DoTally
- Consolidation
- TiesResolution
- ComputeResult
- GenerateReport

We can represent them using an `enum`:

```rust
enum Pipe {
    DecodeBallots,
    DoTally,
    Consolidation,
    TiesResolution,
    ComputeResult,
    GenerateReport,
}
```

The Pipe enum can deserialize the value into `VelvetDecodeBallots`, using `Velvet` as a prefix for namespace, in case where we implement pipes from another modules.

For each pipe, we implement a `struct` that implements a `trait`.

```rust
trait Pipe {
    // pipe execution
    fn exec(&self) -> Result<()> {
        dbg!(&self.config);
        dbg!(&self.input_dir);
        dbg!(&self.output_dir);

        // file handle to log execution process into
        dbg!(&self.output_log_file);

        Ok(())
    }

    // load input
    fn input(&self) -> Result<()>;

    // produce output
    fn output(&self) -> Result<()>;
}
```

### DecodeBallots

todo

### DoTally

Ballots inputs are stored in a `ballot__<uuid>.csv` file where the data is `\n` separated.

Each line represent a ballot to be tallied.

The _DoTally pipe_ will take that in consideration and produce the count for the particular contest for a particular area, within an election.

The pipe also take in consideration the election configuration that is given as an input configuration file.

```
./path/to/input-dir/election__<uuid>/config.json
```

#### Configuration

##### Invalid ballots

TODO: determine if the invalid ballots configuration should be set in the `velvet-config.json` for the `VelvetDoTally` pipe or in the election configuration.

Invalid ballots can be represented as such:

```rust
enum InvalidBallot {
    Blank,
    ExplicitInvalid(InvalidBallotReason),
    ImplicitInvalid(InvalidBallotReason),
}

enum InvalidBallotReason {
    MarkedAsInvalid,
    NoCandidate,
    InvalidCandidate,
    BallotCorrupted,
}
```

### Other Pipes

todo
