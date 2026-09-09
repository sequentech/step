# Local trustee configurations

Trustee secrets are generated per environment and kept outside Git. Before using
the legacy development air-gap packaging script, provision each trustee with the
`gen_trustee_config` binary from the selected build:

```bash
umask 077
for trustee in trustee1 trustee2 trustee3; do
    mkdir -p ".devcontainer/trustees-data/$trustee"
    # Refuse to replace keys belonging to an existing board or election.
    (set -o noclobber; gen_trustee_config > ".devcontainer/trustees-data/$trustee/$trustee.toml") || exit 1
done
```

Run from the repository root. Preserve the resulting files with the associated
environment and register their public keys before use. Do not regenerate them to
recover an existing election; restore its matching keys. The evaluated CM delivery
procedure has its own protected package and custody records.
