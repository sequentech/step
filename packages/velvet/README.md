<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Velvet

**Velvet** -- A type of woven tufted fabric, implying smoothness and luxury.

You can call velvet from the command line like this:

```
$> velvet run {stage} {optional-pipe} \
  --config ./path/to/velvet-config.json \
  --input-dir ./path/to/input-dir \
  --output-dir ./path/to/output-dir
```

See more: 

https://github.com/sequentech/step/blob/main/docs/design/velvet/README.md

## Development

For example if you are testing a specific vote receipts template, you could do:

```bash
cargo build
# update velvet config with vote receipts template
python3 testing/bin/utils/update_velvet_config.py \
    --config-path ./velvet-config.json \
    --template-path ./src/resources/vote_receipts.hbs && \

# run velvet
rm -rf output2 && \
cargo run --bin velvet -- run \
    main decode-ballots \
    --config ./velvet-config.json \
    --input-dir ./input \
    --output-dir ./output2
```
