# Trustee configuration

Generate each local trustee configuration using the `gen_trustee_config` binary
from the selected build. For example, from this directory:

```bash
umask 077
(set -o noclobber; gen_trustee_config > trustee1.toml)
```

Repeat for each distinct trustee before registering its public key. The files are
ignored by Git. Keep existing keys for the board/election that uses them; restore
the original configuration for recovery. Never replace it with a newly generated key.
