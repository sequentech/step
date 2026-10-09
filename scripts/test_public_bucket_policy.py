# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Execute MinIO configuration with synthetic credentials and a captured policy."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]


class PublicBucketPolicyTests(unittest.TestCase):
    def configure(self, fail_policy=False):
        with tempfile.TemporaryDirectory() as directory:
            staging = Path(directory)
            capture = staging / "policy.json"
            mc = staging / "mc"
            mc.write_text(f"#!{sys.executable}\n" + r'''import json
import os
from pathlib import Path
import sys
args = sys.argv[1:]
if args[:2] == ["anonymous", "set-json"]:
    if os.environ.get("FAIL_PUBLIC_POLICY") == "true":
        sys.exit(44)
    policy = sys.stdin.read() if args[2] == "/dev/stdin" else Path(args[2]).read_text()
    Path(os.environ["MINIO_POLICY_CAPTURE"]).write_text(policy)
elif args[:3] == ["anonymous", "set", "download"]:
    if os.environ.get("FAIL_PUBLIC_POLICY") == "true":
        sys.exit(44)
    # The download preset also grants anonymous ListBucket.
    Path(os.environ["MINIO_POLICY_CAPTURE"]).write_text(json.dumps({
        "Version": "2012-10-17",
        "Statement": [{"Effect": "Allow", "Principal": {"AWS": ["*"]},
                       "Action": ["s3:GetObject", "s3:ListBucket"],
                       "Resource": ["arn:aws:s3:::test-public", "arn:aws:s3:::test-public/*"]}]
    }))
''')
            mc.chmod(0o755)
            env = {
                **os.environ,
                "PATH": str(staging) + os.pathsep + (os.environ.get("PATH") or os.defpath),
                "MINIO_POLICY_CAPTURE": str(capture),
                "TMPDIR": str(staging),
                "FAIL_PUBLIC_POLICY": str(fail_policy).lower(),
                "MINIO_PRIVATE_URI": "http://example.invalid:9000",
                "MINIO_ROOT_USER": "synthetic-user",
                "MINIO_ROOT_PASSWORD": "synthetic-password",
                "MINIO_ACCESS_KEY": "synthetic-access",
                "MINIO_ACCESS_SECRET": "synthetic-secret",
                "MINIO_PUBLIC_BUCKET": "test-public",
                "MINIO_BUCKET": "test-private",
                "KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY": "tenant.json",
                "KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY": "event.json",
            }
            result = subprocess.run(
                ["bash", str(ROOT / ".devcontainer/minio/entrypoint.sh")],
                env=env, capture_output=True, text=True, timeout=30,
            )
            policy = json.loads(capture.read_text()) if capture.exists() else None
            return result, policy

    def test_public_bucket_allows_object_reads_without_listing(self):
        result, policy = self.configure()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNotNone(policy)
        self.assertEqual(policy["Version"], "2012-10-17")
        self.assertEqual(policy["Statement"], [{
            "Effect": "Allow", "Principal": {"AWS": ["*"]},
            "Action": ["s3:GetObject"], "Resource": ["arn:aws:s3:::test-public/*"],
        }])

    def test_configuration_works_without_an_inherited_path(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            result, policy = self.configure()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNotNone(policy)

    def test_configuration_fails_when_public_policy_cannot_be_applied(self):
        result, policy = self.configure(fail_policy=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(policy)


if __name__ == "__main__":
    unittest.main()
