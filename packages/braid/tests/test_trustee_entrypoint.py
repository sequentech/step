"""Entrypoint behavior in an offline Ubuntu 26.04 container with explicit test binaries.

Run after importing the Ubuntu image: python3 -m unittest discover -s packages/braid/tests
These checks do not build or substitute for tests of the trustee protocol itself.
"""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/trustee.sh"


class Entrypoint(unittest.TestCase):
    def run_entrypoint(self, *, environment=(), mounted=False, bao=False):
        with tempfile.TemporaryDirectory(prefix="trustee-entrypoint-") as root:
            root = Path(root)
            for name, body in {
                "gen_trustee_config": "echo GENERATED >&2; printf 'key_pk = public\\nkey_sk = PRIVATE-SENTINEL\\n'",
                "trustee": "echo TRUSTEE-STARTED; test -s /opt/braid/trustee.toml",
                **({"bao": "echo FETCH-FAILED >&2; exit 1"} if bao else {}),
            }.items():
                file = root / name
                file.write_text("#!/bin/sh\n" + body + "\n")
                file.chmod(0o755)
            command = ["docker", "run", "--rm", "--pull=never", "--network=none",
                       "-v", f"{SCRIPT}:/test/trustee.sh:ro", "-v", f"{root}:/test/bin:ro",
                       "-e", "PATH=/test/bin:/usr/bin:/bin", "-e", "TRUSTEE_NAME=test"]
            for variable in environment:
                command += ["-e", variable]
            script = "mkdir -p /opt/braid; "
            if mounted:
                script += "printf 'key_pk = public\\nkey_sk = PRIVATE-SENTINEL\\n' > /opt/braid/trustee.toml; "
            script += "bash /test/trustee.sh"
            return subprocess.run(command + [os.environ.get("TRUSTEE_TEST_IMAGE", "ubuntu:26.04"), "bash", "-c", script],
                                  capture_output=True, text=True)

    def test_mounted_configuration_needs_no_cloud_tools_and_does_not_log_private_key(self):
        result = self.run_entrypoint(mounted=True, environment=["SECRETS_BACKEND=AwsSecretsManager"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("TRUSTEE-STARTED", result.stdout)
        self.assertNotIn("PRIVATE-SENTINEL", result.stdout + result.stderr)
        self.assertNotIn("GENERATED", result.stderr)

    def test_default_requires_persistent_configuration(self):
        result = self.run_entrypoint()
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("GENERATED", result.stderr)
        self.assertNotIn("TRUSTEE-STARTED", result.stdout)

    def test_cloud_backend_is_disabled_before_key_generation(self):
        result = self.run_entrypoint(environment=["SECRETS_BACKEND=HashicorpVault"])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("disabled", result.stderr)
        self.assertNotIn("GENERATED", result.stderr)

    def test_optional_vault_fetch_failure_never_replaces_a_key(self):
        result = self.run_entrypoint(environment=["SECRETS_BACKEND=HashicorpVault", "CLOUD_SECRET_BACKENDS=enabled"], bao=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("GENERATED", result.stderr)
        self.assertNotIn("TRUSTEE-STARTED", result.stdout)

    def test_ephemeral_keys_require_explicit_test_mode(self):
        result = self.run_entrypoint(environment=["TRUSTEE_ALLOW_EPHEMERAL=true"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("GENERATED", result.stderr)
        self.assertNotIn("PRIVATE-SENTINEL", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
