import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "initialize-local-secrets.py"
spec = importlib.util.spec_from_file_location("local_secrets", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


@unittest.skipUnless(shutil.which("openssl"), "OpenSSL is required for development certificates")
class LocalCredentialsTests(unittest.TestCase):
    def test_fresh_installations_differ_and_reinitialization_preserves_state(self):
        with tempfile.TemporaryDirectory() as temporary:
            roots = [Path(temporary) / name for name in ("first", "second")]
            identities = []
            for root in roots:
                root.mkdir()
                (root / ".env.development").write_text("AWS_S3_ACCESS_KEY=\nCUSTOM=value\n")
                module.initialize(root)
                paths = [root / ".env", root / "certs/nginx-tls.key", root / "certs/nginx-tls.crt",
                         root / "simplesamlphp/cert/server.pem", root / "simplesamlphp/cert/server.crt",
                         root / "certs/vp-sso-signing.key", root / "certs/vp-sso-signing.crt"]
                before = [p.read_bytes() for p in paths]
                self.assertEqual(paths[0].stat().st_mode & 0o777, 0o600)
                self.assertEqual(paths[1].stat().st_mode & 0o777, 0o600)
                module.initialize(root)
                self.assertEqual([p.read_bytes() for p in paths], before)
                env = dict(line.split("=", 1) for line in paths[0].read_text().splitlines() if "=" in line)
                self.assertEqual(env["CUSTOM"], "value")
                self.assertEqual(len({env[k] for k in module.GENERATED}), len(module.GENERATED))
                for name in module.GENERATED:
                    self.assertRegex(env[name], "^[0-9a-f]{20}$" if name == "AWS_S3_ACCESS_KEY" else "^[0-9a-f]{48}$")
                public = "".join(line for line in paths[4].read_text().splitlines() if not line.startswith("---"))
                self.assertEqual(env["SSP_SIGNING_CERTIFICATE"], public)
                client_public = "".join(line for line in paths[6].read_text().splitlines() if not line.startswith("---"))
                self.assertEqual(env["KEYCLOAK_SAML_CLIENT_SIGNING_CERTIFICATE"], client_public)
                for key, cert in [(paths[1], paths[2]), (paths[3], paths[4]), (paths[5], paths[6])]:
                    actual = subprocess.check_output(["openssl", "pkey", "-in", str(key), "-pubout"])
                    expected = subprocess.check_output(["openssl", "x509", "-in", str(cert), "-pubkey", "-noout"])
                    self.assertTrue(actual == expected, "certificate does not match generated key")
                identities.append(before)
            self.assertTrue(all(a != b for a, b in zip(*identities)), "installations share generated material")

    def test_incomplete_existing_pair_is_preserved_and_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            key = root / "nginx.key"
            key.write_text("existing-key-must-survive")
            with self.assertRaisesRegex(ValueError, "incomplete certificate pair"):
                module.certificate(root, key.name, "nginx.crt", "localhost", "DNS:localhost")
            self.assertEqual(key.read_text(), "existing-key-must-survive")
            self.assertFalse((root / "nginx.crt").exists())


if __name__ == "__main__":
    unittest.main()
