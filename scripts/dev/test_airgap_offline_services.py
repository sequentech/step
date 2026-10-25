# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""The generated air-gapped Compose bundle: its offline services, simulated
messaging, the Scanovate images it always carries and the choice to start
them, and the secrets `up` generates instead of the bundle carrying them.

    python3 -m unittest scripts/dev/test_airgap_offline_services.py
"""

import hashlib
import json
import os
import re
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILES = ROOT / "scripts" / "airgap-files"
BUILD = (ROOT / "scripts" / "build-airgapped.sh").read_text()
UP = (FILES / "up").read_text()

SCANOVATE = {
    "scanovate-ocr": "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/ocr@sha256:b305c844377682e33866cdca0b43a80249ef175697e885301f4026f0975c2246",
    "scanovate-face-match": "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/ngfacematch@sha256:13dadfa795f5b52667b488ad0d3d248dcf9f4bf825e5f44ff90436ddb8c178e7",
    "scanovate-liveness": "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/liveness-plus-service@sha256:75500fbeefefa3417ba38838daf222d6a9fb1460970df3c5da853e8813200b49",
    "scanovate-presentation-detection": "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/liveness-presentation-detection-service@sha256:56d62785f400536085eaf6b4dfecb5fa42e462c95c2e2c43120b7c7a37fb501b",
}
SECRETS = [
    "MESSAGING_RELAY_SECRET",
    "MONITORING_RENDERER_TOKEN",
    "SCANOVATE_LIVENESS_JWT_SECRET_KEY",
    "SCANOVATE_OCR_JWT_SECRET_KEY",
    "SCANOVATE_LIVENESS_CALLBACK_SECRET",
]


def substituted(text):
    """build-airgapped.sh's own substitution."""
    return text.replace("STEP_VERSION", "v0.0.0-test").replace("STEP_HASH", "0" * 7)


def env_value(env, key):
    match = re.search(rf'^{key}="?([^"\n]*)"?$', env, re.M)
    return match.group(1) if match else None


class BundleCompose(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.env = (FILES / ".env").read_text()

    @unittest.skipUnless(shutil.which("docker"), "Docker Compose is required")
    def test_the_generated_bundle_renders_with_every_offline_service(self):
        with tempfile.TemporaryDirectory() as tmp:
            project = Path(tmp)
            (project / "docker-compose.yml").write_text(substituted((FILES / "docker-compose.yml").read_text()))
            (project / ".env").write_text(substituted(self.env))
            result = subprocess.run(
                ["docker", "compose", "--project-directory", tmp, "--env-file", str(project / ".env"),
                 "--profile", "*", "config", "--format", "json"],
                capture_output=True, text=True,
                env={k: os.environ[k] for k in ("PATH", "HOME") if k in os.environ},
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            services = json.loads(result.stdout)["services"]
        for name in ["monitoring-renderer", "messaging-webhooks", *SCANOVATE]:
            self.assertIn(name, services)
        for name, image in SCANOVATE.items():
            self.assertEqual(services[name]["image"], image)
            # Started only with the biometric profile, never by `full`.
            self.assertEqual(services[name]["profiles"], ["biometric"])
            self.assertEqual(services[name].get("pull_policy"), "never")
        for name in ["monitoring-renderer", "messaging-webhooks"]:
            self.assertIn("healthcheck", services[name], name)
        harvest = services["harvest"]["environment"]
        self.assertEqual(harvest["MESSAGING_FAKE_PROVIDER_ROUTES"], "ENABLED")
        for sender in ["harvest", "windmill", "beat"]:
            environment = services[sender]["environment"]
            for key in ["MESSAGING_META_GRAPH_URL", "MESSAGING_INFOBIP_URL", "MESSAGING_AWS_ENDPOINT_URL"]:
                self.assertTrue(environment[key].startswith("http://harvest:8400/fake-provider/"), (sender, key))
            self.assertEqual(environment["MESSAGING_WEBHOOKS_PUBLIC_URL"], "http://messaging-webhooks:8410")

    def test_the_bundle_carries_no_secret_only_placeholders_up_replaces(self):
        for key in SECRETS:
            self.assertEqual(env_value(self.env, key), "generate-on-first-up", key)
        self.assertNotIn("change-me", self.env.lower())
        template = (FILES / "scanovate" / "liveness" / "service_config.json.tmpl").read_text()
        self.assertEqual(template.count("__LIVENESS_CALLBACK_SECRET__"), 2)
        self.assertNotIn("liveness-dev-callback-secret", template)

    def test_simulated_messaging_is_only_in_the_offline_bundle(self):
        self.assertEqual(env_value(self.env, "MESSAGING_FAKE_PROVIDER_ROUTES"), "ENABLED")
        compose = (FILES / "docker-compose.yml").read_text()
        # The bundle's compose defaults it off; only the bundle's .env enables it.
        self.assertIn("MESSAGING_FAKE_PROVIDER_ROUTES: ${MESSAGING_FAKE_PROVIDER_ROUTES:-DISABLED}", compose)
        for other in [ROOT / ".devcontainer" / ".env.remote-deployment.example",
                      ROOT / ".devcontainer" / "docker-compose-airgap-preparation.yml"]:
            self.assertNotIn("MESSAGING_FAKE_PROVIDER_ROUTES=\"ENABLED\"", other.read_text(), other)

    def test_the_scanovate_images_are_not_pulled_when_preparing(self):
        prep = (ROOT / ".devcontainer" / "docker-compose-airgap-preparation.yml").read_text()
        self.assertNotIn("scanovate", prep)


UP_PRELUDE = UP[: UP.index("# Add two optional input params")]


def run_up_prelude(directory, env_text, biometric=None, containerd=True):
    """Run `up`'s secret and biometric steps in `directory`, with a stub docker."""
    directory = Path(directory)
    (directory / ".env").write_text(env_text)
    (directory / "scanovate" / "liveness").mkdir(parents=True, exist_ok=True)
    shutil.copy2(FILES / "scanovate" / "liveness" / "service_config.json.tmpl",
                 directory / "scanovate" / "liveness" / "service_config.json.tmpl")
    if biometric is not None:
        (directory / "biometric.json").write_text(json.dumps({"biometric": biometric}))
    stubs = directory / "stubs"
    stubs.mkdir(exist_ok=True)
    driver = "[[driver-type io.containerd.snapshotter.v1]]" if containerd else "[[Backing Filesystem extfs]]"
    (stubs / "docker").write_text(f"#!/bin/sh\necho '{driver}'\n")
    (stubs / "docker").chmod(0o755)
    script = UP_PRELUDE + '\necho "PROFILES=${PROFILES[*]}"\n'
    return subprocess.run(["bash", "-c", script], cwd=directory, capture_output=True, text=True,
                          env={"PATH": f"{stubs}:{os.environ['PATH']}"})


class UpPrelude(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.env = substituted((FILES / ".env").read_text())

    def test_secrets_are_generated_once_and_kept(self):
        result = run_up_prelude(self.tmp, self.env.replace('BIOMETRIC_SERVICES="start"', 'BIOMETRIC_SERVICES="stopped"'))
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        first = (self.tmp / ".env").read_text()
        for key in SECRETS:
            self.assertRegex(env_value(first, key), r"^[0-9a-f]{64}$", key)
            self.assertNotIn(env_value(first, key), result.stdout)
        # A second run keeps them.
        run_up_prelude(self.tmp, first)
        self.assertEqual(first, (self.tmp / ".env").read_text())

    def test_stopped_biometrics_are_not_started_and_say_so(self):
        result = run_up_prelude(self.tmp, self.env.replace('BIOMETRIC_SERVICES="start"', 'BIOMETRIC_SERVICES="stopped"'),
                                biometric="included")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("NOT STARTED", result.stdout)
        self.assertIn("PROFILES=--profile full\n", result.stdout)

    def test_started_biometrics_render_their_configuration(self):
        result = run_up_prelude(self.tmp, self.env, biometric="included")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        self.assertIn("PROFILES=--profile full --profile biometric", result.stdout)
        secret = env_value((self.tmp / ".env").read_text(), "SCANOVATE_LIVENESS_CALLBACK_SECRET")
        config = (self.tmp / "scanovate" / "liveness" / "service_config.json").read_text()
        self.assertEqual(config.count(f"secret={secret}"), 2)
        self.assertNotIn(secret, result.stdout)

    def test_a_partial_bundle_never_starts_biometrics(self):
        result = run_up_prelude(self.tmp, self.env, biometric="absent")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("DEVELOPER PARTIAL", result.stdout)
        result = run_up_prelude(self.tmp, self.env)  # no biometric.json at all
        self.assertNotEqual(result.returncode, 0)

    def test_digest_named_images_need_the_containerd_store(self):
        result = run_up_prelude(self.tmp, self.env, biometric="included", containerd=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("containerd image store", result.stdout)


BLOCK = BUILD[BUILD.index("# --- Scanovate (biometric) images: begin ---"):
              BUILD.index("# --- Scanovate (biometric) images: end ---")]


def blob(layout, data):
    digest = hashlib.sha256(data).hexdigest()
    (layout / "blobs" / "sha256" / digest).write_bytes(data)
    return "sha256:" + digest


def synthetic_layout(root, count=4):
    """An OCI layout of `count` synthetic images; returns it and their refs."""
    layout = root / "layout"
    (layout / "blobs" / "sha256").mkdir(parents=True)
    (layout / "oci-layout").write_text('{"imageLayoutVersion":"1.0.0"}')
    refs, entries = [], []
    for i in range(count):
        config = blob(layout, json.dumps({"architecture": "amd64", "os": "linux", "n": i}).encode())
        layer = blob(layout, f"layer {i}".encode())
        manifest = json.dumps({
            "schemaVersion": 2, "mediaType": "application/vnd.oci.image.manifest.v1+json",
            "config": {"mediaType": "application/vnd.oci.image.config.v1+json", "digest": config, "size": 1},
            "layers": [{"mediaType": "application/vnd.oci.image.layer.v1.tar", "digest": layer, "size": 1}],
        }).encode()
        digest = blob(layout, manifest)
        refs.append(f"example.test/scanovate/image{i}@{digest}")
        entries.append({"mediaType": "application/vnd.oci.image.manifest.v1+json", "digest": digest,
                        "size": len(manifest)})
    (layout / "index.json").write_text(json.dumps({"schemaVersion": 2, "manifests": entries}))
    return layout, refs


def run_block(script, refs, env):
    images = " ".join(f'"{r}"' for r in refs)
    full = "info() { echo \"(info) $1\"; }\n" + BLOCK + f"\nSCANOVATE_IMAGES=({images})\n" + script
    return subprocess.run(["bash", "-c", full], capture_output=True, text=True,
                          env={"PATH": os.environ["PATH"], **env})


class ScanovateImages(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.layout, self.refs = synthetic_layout(self.tmp)
        self.check = 'verify-scanovate-images && echo "BIOMETRIC=$BIOMETRIC"\n'

    def test_the_build_pins_exactly_the_approved_digests(self):
        self.assertEqual(sorted(re.findall(r'"(us-east1-docker\.pkg\.dev/[^"]+)"', BLOCK)), sorted(SCANOVATE.values()))

    def test_a_complete_layout_is_verified_and_archived_named_by_digest(self):
        archive = self.tmp / "scanovate-oci.tar"
        result = run_block(self.check + f'write-scanovate-archive "{archive}"\n', self.refs,
                           {"SCANOVATE_OCI_DIR": str(self.layout)})
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        self.assertIn("BIOMETRIC=included", result.stdout)
        with tarfile.open(archive) as tar:
            index = json.load(tar.extractfile("index.json"))
            names = set(tar.getnames())
        self.assertEqual(sorted(m["annotations"]["io.containerd.image.name"] for m in index["manifests"]),
                         sorted(self.refs))
        for ref in self.refs:
            self.assertIn("blobs/sha256/" + ref.split("@sha256:")[1], names)

    def test_a_tampered_blob_is_refused(self):
        manifest = json.loads((self.layout / "blobs" / "sha256" / self.refs[2].split(":")[-1]).read_text())
        layer = self.layout / "blobs" / "sha256" / manifest["layers"][0]["digest"].split(":")[1]
        layer.write_bytes(b"tampered")
        result = run_block(self.check, self.refs, {"SCANOVATE_OCI_DIR": str(self.layout)})
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("BIOMETRIC=included", result.stdout)
        self.assertIn("does not match its digest", result.stderr)

    def test_a_missing_image_is_refused(self):
        missing = self.refs + ["example.test/scanovate/absent@sha256:" + "0" * 64]
        result = run_block(self.check, missing, {"SCANOVATE_OCI_DIR": str(self.layout)})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("is not in", result.stderr)

    def test_no_layout_refuses_a_full_bundle_and_only_an_opt_in_makes_a_partial_one(self):
        refused = run_block(self.check, self.refs, {})
        self.assertNotEqual(refused.returncode, 0)
        self.assertNotRegex(refused.stdout, r"(?m)^BIOMETRIC=")
        self.assertIn("full offline bundle carries", refused.stderr)
        partial = run_block(self.check, self.refs, {"ALLOW_MISSING_BIOMETRIC": "1"})
        self.assertEqual(partial.returncode, 0, partial.stderr)
        self.assertIn("DEVELOPER PARTIAL", partial.stdout)
        self.assertIn("BIOMETRIC=absent", partial.stdout)


if __name__ == "__main__":
    unittest.main()
