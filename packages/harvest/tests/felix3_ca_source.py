# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline checks on the opt-in assessment assembly. Run directly with Python 3.11 and PyYAML."""

from pathlib import Path
import re
import tomllib
import unittest

import yaml


HARVEST = Path(__file__).resolve().parents[1]
FEATURE = "felix3-local-initialize-ca"
PRIVATE_PATH = "/private/v1/operational-signing/initialize/ca"


class AssessmentAssembly(unittest.TestCase):
    def test_ordinary_and_sealed_startup_have_no_dependency_on_the_ca_binary(self):
        cargo = tomllib.loads((HARVEST / "Cargo.toml").read_text())
        features = cargo["features"]
        self.assertNotIn(FEATURE, features.get("default", []))
        for name in ("felix3-local-initialize", "operational-signing-demo-fixture"):
            self.assertNotIn(FEATURE, features[name])
        target = next(b for b in cargo["bin"] if b["name"] == "harvest-felix3-initialize-ca")
        self.assertEqual(target["required-features"], [FEATURE])
        binary = (HARVEST / target["path"]).read_text()
        self.assertIn("harvest::local_initialize_ca::serve()", binary)
        self.assertNotIn("harvest::rocket()", binary)
        primary = (HARVEST / "src/lib.rs").read_text().split("pub async fn rocket()", 1)[1]
        self.assertNotIn("local_initialize_ca", primary)
        self.assertNotIn("private_operational_ca", primary)
        self.assertNotIn(PRIVATE_PATH, primary)

    def test_overlay_has_only_the_private_bridge_and_read_only_role_mounts(self):
        overlay = yaml.safe_load((HARVEST / "compose.felix3-local-initialize-ca.yaml").read_text())
        self.assertEqual(set(overlay), {"services"})
        self.assertEqual(set(overlay["services"]), {"felix3-initialize-ca"})
        ca = overlay["services"]["felix3-initialize-ca"]
        for option in ("ports", "expose", "network_mode", "privileged", "env_file", "build"):
            self.assertNotIn(option, ca)
        self.assertEqual(ca["profiles"], ["felix3-local-initialize"])
        self.assertEqual(set(ca["networks"]), {"felix3-device-ca"})
        address = ca["networks"]["felix3-device-ca"]["ipv4_address"]
        self.assertEqual(ca["environment"], {"FELIX3_CA_LISTEN_IP": address})
        self.assertEqual(ca["entrypoint"], ["/usr/local/bin/harvest-felix3-initialize-ca"])
        root = Path("/run/sequent-felix3-local-initialize")
        mounts = {Path(v["target"]).relative_to(root).as_posix(): v for v in ca["volumes"]}
        self.assertEqual(set(mounts), {"installation.json", "ca", "postgres"})
        self.assertTrue(all(v["read_only"] is True for v in mounts.values()))
        self.assertEqual(mounts["postgres"]["source"], "felix3-app-postgres-socket")
        for name in ("ca", "installation.json"):
            self.assertIs(mounts[name]["bind"]["create_host_path"], False)

    def test_only_the_existing_verified_assessment_can_return_a_certificate(self):
        route = (HARVEST / "src/routes/private_operational_ca.rs").read_text()
        self.assertEqual(re.findall(r'rocket::post\(\s*"([^"]+)"', route), [PRIVATE_PATH])
        self.assertEqual(re.findall(r'\.mount\("/", rocket::routes!\[([^\]]+)\]\)', route), ["assess"])
        self.assertIn("peer: Certificate<'_>", route)
        self.assertIn("MutualTls::from_bytes(approved_device_client_ca).mandatory(true)", route)
        first = route.index("pin != state.peer_pin")
        before = route.index("state.local.authorize", first)
        check = route.index("assess_reserved_ca(&tx", before)
        after = route.index("state.local.authorize", check)
        self.assertLess(after, route.index("result.map(Json)"))
        adapter = (HARVEST / "src/local_initialize_ca/mod.rs").read_text()
        self.assertLess(adapter.index("scope.pool(root).await"), adapter.index("let app = unmounted_authority"))
        self.assertIn("marker == Some(i.installation)", adapter)
        self.assertIn("allowed != Some(true)", adapter)

    def test_certificate_paths_are_checked_against_the_installed_transport_roots(self):
        tls = (HARVEST / "src/local_initialize_ca/transport.rs").read_text()
        for guard in (
            "certificates.len() == 1", "actual == expected", "is_ca(&cert)?",
            "server_spki == registration.server_spki_sha256",
            "leaf.public_key()?.public_eq(&private_key)", "store.add_cert(server_ca)?",
            "param.set_purpose(X509PurposeId::SSL_SERVER)", "param.set_ip(IpAddr::V4(*registration.listen.ip()))",
            "ctx.verify_cert()", "sent.to_der()? != verified.to_der()?",
            "client_ca: client_ca.to_pem()?", "device_spki: pins.device_to_ca.0",
        ):
            self.assertIn(guard, tls)
        self.assertNotIn("set_default_paths", tls)

    def test_malformed_enabled_primary_registration_cannot_take_the_inactive_exit(self):
        local = (HARVEST / "src/routes/local_initialize_authority.rs").read_text()
        mount = local.split("pub(crate) async fn mount(", 1)[1].split("#[cfg(test)]", 1)[0]
        self.assertRegex(mount, r"if !installation_present\(root\)\?\s*\{\s*return Ok\(application\);")
        self.assertEqual(mount.count("return Ok(application)"), 1)
        self.assertIn('load(&application, root).context(', mount)
        self.assertIn("ensure!(install.post.present(&tx).await?", mount)

    def test_probe_has_no_table_grants_and_the_post_check_is_request_bound(self):
        sql = (HARVEST / "sql/felix3_local_initialize_ca.sql").read_text()
        self.assertEqual(sql.count("SECURITY DEFINER"), 1)
        grants = re.findall(r"^GRANT\s+(.+?);", sql, re.I | re.M | re.S)
        for grant in grants:
            if re.search(r"\bTO\b.*\bfelix3_local_initialize_ca\b", grant, re.I | re.S):
                self.assertTrue(grant.startswith("EXECUTE ON FUNCTION"))
        for required in (
            "p_tenant, p_event, p_post, p_publication, 'softhsm-test') = p_run",
            "ORDER BY published_at DESC, id LIMIT 1",
            "(p_tenant, p_event, p_post, p_request)",
            "(p_official::text, p_challenge, p_fence)",
            "r.subject->>'publication_id' = p_publication::text",
            "(i.binding).payload_hash = r.payload_sha256",
            "'request_id', p_request, 'subject', jsonb_build_object('publication_id', p_publication)",
        ):
            self.assertIn(required, sql)
        role = (HARVEST / "src/local_initialize_ca/principal.sql").read_text()
        for check in ("current_user = session_user", "inet_client_addr() IS NULL", "admin_option",
                      "'sequent_os_initialize_ca','USAGE'", "has_any_column_privilege",
                      "has_sequence_privilege", "has_function_privilege"):
            self.assertIn(check, role)


if __name__ == "__main__":
    unittest.main()
