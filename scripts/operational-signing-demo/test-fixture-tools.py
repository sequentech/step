#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import http.client
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
import uuid

import sys
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class FixtureTests(unittest.TestCase):
    def test_normal_audience_and_two_distinct_flows(self):
        module = load("realm", "keycloak/render-realm.py")
        key = uuid.uuid4()
        realm = module.render(key, "z" * 32)
        gold, portal = realm["clients"]
        self.assertEqual(realm["realm"], f"tenant-{key}")
        self.assertFalse(gold["publicClient"])
        self.assertTrue(portal["publicClient"])
        self.assertIn("basic", portal["defaultClientScopes"])
        self.assertIn("basic", gold["defaultClientScopes"])
        self.assertEqual(gold["attributes"]["require.pushed.authorization.requests"], "true")
        self.assertEqual(gold["attributes"]["minimum.acr.value"], "gold")
        self.assertEqual(gold["redirectUris"], ["https://127.0.0.1:13443/operational-signing/return"])
        self.assertFalse(gold["directAccessGrantsEnabled"] or portal["directAccessGrantsEnabled"])
        self.assertFalse(gold["serviceAccountsEnabled"] or portal["serviceAccountsEnabled"])
        self.assertNotEqual(gold["authenticationFlowBindingOverrides"], portal["authenticationFlowBindingOverrides"])
        self.assertEqual(portal["protocolMappers"][0]["config"]["included.client.audience"], "admin-portal")
        self.assertEqual(realm["authenticationFlows"][3]["authenticationExecutions"][1]["authenticator"], "auth-otp-form")
        by_alias = {flow["alias"]: flow for flow in realm["authenticationFlows"]}
        browser = realm["authenticationFlows"][2]["authenticationExecutions"]
        self.assertEqual(browser[0]["authenticator"], "auth-cookie")
        self.assertNotIn("auth-cookie", str(realm["authenticationFlows"][:2]))
        normal = by_alias[browser[1]["flowAlias"]]["authenticationExecutions"]
        self.assertEqual([by_alias[e["flowAlias"]]["authenticationExecutions"][1]["authenticator"]
                          for e in normal], ["auth-username-password-form", "auth-otp-form"])
        self.assertTrue(all(e["requirement"] == "CONDITIONAL" for e in normal))
        with self.assertRaises(ValueError):
            module.render(uuid.UUID(int=0), "z" * 32)

    def test_public_crl_only(self):
        module = load("crl", "serve-fixture-crl.py")
        def openssl(*args):
            return subprocess.run(["openssl", *args], check=True, cwd=state,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5).stdout
        with tempfile.TemporaryDirectory(prefix="synthetic-demo-crl-") as tmp:
            state = Path(tmp).resolve()
            (state / "index").touch()
            (state / "number").write_text("0100\n")
            (state / "ca.conf").write_text("""[ca]
default_ca=issuer
[issuer]
database=index
certificate=ca.pem
private_key=ca.key
crlnumber=number
default_md=sha256
default_crl_days=1
""")
            openssl("req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                    "-nodes", "-subj", "/CN=SYNTHETIC CRL ONLY", "-days", "1", "-keyout", "ca.key", "-out", "ca.pem")
            openssl("ca", "-batch", "-config", "ca.conf", "-gencrl", "-out", "crl.pem")
            with module.HTTPServer(("127.0.0.1", 0), module.handler(state)) as server:
                task = threading.Thread(target=server.serve_forever, daemon=True)
                task.start()
                try:
                    def get(path):
                        with_connection = http.client.HTTPConnection("127.0.0.1", server.server_port, timeout=3)
                        try:
                            with_connection.request("GET", path)
                            r = with_connection.getresponse()
                            return r.status, r.read()
                        finally:
                            with_connection.close()

                    status, receipt = get(module.URL_PATH)
                    self.assertEqual(status, 200)
                    self.assertEqual(receipt, openssl("crl", "-in", "crl.pem", "-outform", "DER"))
                    for name in ("/", "/ca.key", "/ca.pem", "/../ca.key", module.URL_PATH + "?file=ca.key"):
                        self.assertEqual(get(name)[0], 404)
                    (state / "crl.pem").write_text("not a revocation list")
                    self.assertEqual(get(module.URL_PATH)[0], 503)
                finally:
                    server.shutdown()
                    task.join(timeout=2)


if __name__ == "__main__":
    unittest.main()
