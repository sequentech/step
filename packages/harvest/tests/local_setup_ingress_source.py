# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline contracts for the normal local Initialize ingress. No application services."""
import json
import re
import sqlite3
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
HARVEST = ROOT / "step/packages/harvest/src/routes"
PORTAL = ROOT / "step/packages/admin-portal/src"
BEYOND = ROOT / "beyond/packages/hsm-service/src"
OVERLAY = ROOT / "step/hasura/felix3-local-initialize"
INGRESS = (HARVEST / "local_setup_ingress.rs").read_text()
STORE = (HARVEST / "local_initialize_self_setup.rs").read_text()
GOLD = (BEYOND / "felix3_local_initialize/setup_gold.rs").read_text()
BROWSER_SQL = (OVERLAY / "1794700002210_local_setup_browser/up.sql").read_text()
SETUP_SQL = (ROOT / "step/hasura/migrations/backend-db/1794700002200_local_initialize_self_setup/up.sql").read_text()


def function(source, name):
    """Take one top-level Rust function up to its unindented closing brace."""
    return re.search(r"\bfn " + name + r"\(.*?\n\}", source, re.S)[0]


def ordered(test, source, *pieces):
    at = 0
    for piece in pieces:
        with test.subTest(after=at, expected=piece):
            test.assertIn(piece, source[at:])
        at = source.index(piece, at) + len(piece)


def wire_fields(source, name):
    body = re.search(r"\bstruct " + name + r"\s*\{([^}]+)\}", source)[1]
    fields = re.findall(r'(?:#\[serde\(rename = "(\w+)"\)\]\s*)?(?:pub\(crate\) )?(\w+): (\w+),', body)
    return {rename or field: kind for rename, field, kind in fields}


class IngressContracts(unittest.TestCase):
    def test_local_feature_and_public_profile_are_opt_in(self):
        for tree in ("step/packages/harvest", "beyond/packages/hsm-service"):
            features = tomllib.loads((ROOT / tree / "Cargo.toml").read_text())["features"]
            self.assertIn("operational-signing-candidate", features["felix3-local-initialize"])
            self.assertNotIn("felix3-local-initialize", features.get("default", []))
            self.assertNotIn("operational-signing-demo-fixture", features["felix3-local-initialize"])
        self.assertEqual(json.loads((PORTAL / "lib/signing/localInitializeProfile.json").read_text()), {"profile": "disabled"})
        routes = (HARVEST / "operational_initialize.rs").read_text()
        self.assertRegex(routes, r'#\[cfg\(all\(feature = "felix3-local-initialize", not\(feature = "operational-signing-demo-fixture"\)\)\)\]\s*#\[path = "local_initialize_self_setup.rs"\]')
        ordered(self, function((HARVEST / "local_initialize_authority.rs").read_text(), "mount"),
                "ensure!(install.post.present(&tx).await?", "SetupServices::load", "Some(setup)", "ingress::register")

    def test_hasura_actions_match_guarded_routes_and_the_browser(self):
        api = (PORTAL / "lib/signing/localInitializeSetupApi.ts").read_text()
        callback = (PORTAL / "lib/signing/finishOperationalApproval.ts").read_text()
        schema = (OVERLAY / "actions.graphql").read_text()
        yaml = (OVERLAY / "actions.yaml").read_text()
        for route, fields in {
            "status": {"request_id", "payload_sha256"},
            "start": {"request_id", "payload_sha256", "attempt", "page"},
            "confirm": {"request_id", "payload_sha256", "attempt", "state", "code"},
            "register": {"request_id", "payload_sha256", "attempt", "delivery_revision"},
        }.items():
            name = "operationalInitializeSelfSetup" + route.title()
            args = r"\b" + name + r"\(([^)]*)\)"
            for source in (schema, callback if route == "confirm" else api):
                self.assertEqual(set(re.findall(r"(\w+)\s*:", re.search(args, source)[1])), fields)
            block = yaml.split("    - name: " + name + "\n", 1)[1].split("\n    - name:", 1)[0].split("custom_types:")[0]
            path = "/operational-signing/initialize/self-setup/" + route
            self.assertIn(path, block)
            self.assertIn('"{{$body.input}}"', block)
            self.assertIn("forward_client_headers: true", block)
            self.assertEqual(re.findall(r"- role: (\S+)", block), ["sign-initialize-voting"])
            self.assertIn('"' + path + '"', INGRESS)
            self.assertIn("session: SetupSession", function(INGRESS, route))
        self.assertNotRegex(api, r"/private/|token_endpoint|reauthWithGold")

    def test_every_private_handoff_rechecks_the_normal_actor_and_post(self):
        guards = function(STORE, "checked_scope")
        ordered(self, guards, "caller(&claims.0)", "prepare_initialize_signing", "pin.verify", "current.payload_sha256()", "tx.commit()")
        authority = (HARVEST / "local_initialize_authority.rs").read_text()
        for predicate in ("self.present(app)", "published_publication_of_post", "read_request", "get_signing_post",
                          "list_signers", "current_signer(&signers", "actor.signs_for"):
            self.assertIn(predicate, authority)
        for name in ("status", "begin", "release"):
            ordered(self, function(STORE, name), "checked_scope(", "felix3_initialize_setup.")
        start = function(INGRESS, "start")
        ordered(self, start, "super::status(", "Phase::SetupAvailable, None", "Uuid::new_v4()", "record_intent(",
                "checked_scope(", "setup.gold.nonce(", "begin(", "Phase::WaitingForSetupGold", "current.original(",
                "checked_scope(", "setup.gold.par(")
        ordered(self, function(STORE, "begin"), "db.transaction()", ".begin_setup({BINDING},$9,$10)",
                "&[&attempt, &broker_nonce_hash]", "if !saved", ".bind_browser({BINDING},$9,$10,$11,$12)",
                "&[&attempt, &session_digest, &path, &panel]", "if !bound", "tx.commit()")

    def test_callback_is_session_scoped_single_use_and_only_confirms(self):
        ordered(self, INGRESS, "OperationalPortalClaims::from_request", 'get_one("Authorization")', "SessionId::id", "Sha256::new()")
        auth = (HARVEST.parent / "services/operational_portal_auth.rs").read_text()
        ordered(self, auth[auth.index("impl<'r> FromRequest"):], "requires_local_ingress()", "pin.accepts(request)",
                'get("Authorization")', "authority.validate(token)")
        confirm = function(INGRESS, "confirm")
        ordered(self, confirm, "super::status(", "before.original(input.attempt)", "Phase::WaitingForSetupGold",
                "checked_scope(", "setup.gold.confirm(", "super::status(", "after.original(", "Phase::SetupConfirmationRequired")
        self.assertNotRegex(confirm, r"\brelease\(|\.deliver\(|approve_operational|Uuid::new_v4")
        ordered(self, GOLD[GOLD.index("pub(crate) async fn confirm"):], "validate_return(state, code)",
                "&nonce, true", ".exchange(", "verify_with_iat(", ".confirm_gold({BINDING},$9,$10,$11,$12,$13,$14,$15,$16)",
                "&attempt, &scope.official, &issuer, &CLIENT, &nonce,", "&authenticated, &issued, &until")
        for term in ("first-use-setup", "scope.official, attempt", "registration.approval_client != CLIENT",
                     "registered_approval_client == CLIENT", "peer.0 != HARVEST"):
            self.assertIn(term, GOLD)

    def test_only_register_can_dispatch_and_only_after_both_commits(self):
        for name in ("status", "start", "confirm"):
            self.assertNotRegex(function(INGRESS, name), r"\brelease\(|\.deliver\(")
        ordered(self, function(INGRESS, "register"), "before.original(input.attempt)", "Phase::SetupConfirmationRequired",
                "input.delivery_revision != owned.revision", "record_intent(", "release(", "checked_scope(",
                "scope != released.scope", "setup.device.deliver(pin, &released)", "super::status(",
                "match (delivery, after.phase)", "(Err(error), _) => Err(error)")
        ordered(self, function(STORE, "release"), "db.transaction()", ".release({BINDING},$9)", "args.push(&attempt)",
                "if !released", ".take_delivery({BINDING},$9,$10,$11)", "&[&session_digest, &reviewed_revision]",
                "if !taken", "tx.commit()", "Ok(ReleasedAttempt")
        self.assertEqual(INGRESS.count(".deliver("), 1)
        self.assertNotIn("Uuid::new_v4", function(INGRESS, "register"))

    def test_gold_binding_and_private_endpoints(self):
        outbound = (HARVEST / "local_setup_transport.rs").read_text()
        inbound = (BEYOND / "felix3_local_initialize/setup_gold_http.rs").read_text()
        fields = {"installation", "tenant", "event", "post", "publication", "request", "official", "payload_sha256"}
        for src in (outbound, inbound):
            binding = re.search(r"struct Binding[^\{]*\{([^}]+)", src)[1]
            self.assertEqual(set(re.findall(r"\b(\w+): ", binding)), fields)
        prefix = re.search(r'const GOLD: &str = "([^"]+)"', outbound)[1]
        for name in ("nonce", "par", "confirm"):
            self.assertIn(prefix + name, inbound)
            self.assertIn("{GOLD}" + name, outbound)
            handler = function(inbound, name)
            self.assertIn("Extension<CallerName>", handler)
            self.assertIn("body.binding.checked(gold.installed_post())", handler)
        for option in ("https_only(true)", "no_proxy()", "Policy::none()", "tls_built_in_root_certs(false)",
                       "pool_max_idle_per_host(0)", "spki(&cert)?", "matches_pushed_redirect("):
            self.assertIn(option, outbound)
        self.assertNotRegex(inbound, r"generate|device\.deliver|\.finish\(")
        broker = (BEYOND / "felix3_local_initialize/setup_broker.rs").read_text()
        ordered(self, broker, "Mount::open(Role::SetupGold)", "pinned_caller(", "database::connect(&files, Role::SetupGold, post)",
                "SetupGold::new(", "TcpListener::bind", "server::serve_router")
        db = (BEYOND / "felix3_local_initialize/database.rs").read_text()
        self.assertIn('Self::SetupGold => "felix3_local_setup_gold"', db)
        self.assertIn("ARRAY['confirm_gold','gold_challenge']", db)

    def test_released_delivery_matches_the_setup_device_and_its_private_receipt(self):
        sender = (HARVEST / "local_setup_transport.rs").read_text()
        device = (BEYOND / "felix3_local_initialize/setup_device.rs").read_text().split("#[cfg(test)]")[0]
        transport = (BEYOND / "felix3_local_initialize/transport.rs").read_text()
        endpoint = "/private/v1/operational-signing/setup/generate"
        self.assertIn(f'const GENERATE: &str = "{endpoint}"', sender)
        self.assertIn(f'.route("{endpoint}", post(generate))', device)
        self.assertIn("bridge(profile.listen, SETUP_DEVICE_PORT)", device)
        self.assertIn("SETUP_DEVICE_PORT: u16 = 22443", transport)
        self.assertIn('Peer::load(root, "device", cfg.device_peer, 22443, &cfg.device_client_spki_sha256)', sender)
        self.assertIn("cfg.gold_peer == cfg.device_peer", sender)
        self.assertIn("cfg.gold_client_spki_sha256 == cfg.device_client_spki_sha256", sender)
        self.assertEqual(wire_fields(sender, "DeviceDelivery"), wire_fields(device, "Delivery"))
        receipt = wire_fields(device, "Enrollment")
        self.assertEqual(receipt.pop("binding"), "Delivery")
        self.assertEqual(wire_fields(sender, "Enrollment"), receipt | {"binding": "DeviceDelivery"})
        self.assertIn('self.action == "initialize-voting"', device)
        self.assertIn("Json(enrollment)", function(device, "generate"))
        ordered(self, function(sender, "deliver"), "released: &ReleasedAttempt", "DeviceDelivery {",
                'action: "initialize-voting"', "challenge: released.attempt", "post(GENERATE, &delivery)",
                "exact_body(response, true)", "enrollment.enrollment_id.is_nil()", "enrollment.binding != delivery",
                "X509Req::from_pem", "public_key_to_der()", "!= enrollment.spki_sha256", "csr.verify(&key)", "Ok(())")
        decoder = (HARVEST / "operational_initialize_broker.rs").read_text().split("async fn exact_body", 1)[1]
        ordered(self, decoder, "StatusCode::OK => {}", "response.chunk()", "MAX_REPLY.saturating_sub", "serde_json::from_slice")
        self.assertNotRegex(INGRESS + (OVERLAY / "actions.graphql").read_text(), r"public_csr_pem|spki_sha256|enrollment_id")

    def test_approval_broker_memberships_match_2100(self):
        db = (BEYOND / "felix3_local_initialize/database.rs").read_text()
        migration = (ROOT / "step/hasura/migrations/backend-db/1794700002100_operational_initialize_local_profile/up.sql").read_text()
        guard = migration.split("HAVING count(DISTINCT m.roleid) = 2", 1)[1].split("THEN", 1)[0]
        groups = re.findall(r"'(\w+)'::regrole", guard)
        self.assertEqual(len(groups), 2)
        broker = re.search(r"Role::Broker => \(&\[([^\]]+)\]", db)[1]
        self.assertCountEqual(re.findall(r'"(\w+)"', broker), groups)
        device = re.search(r"Role::Device => \(&\[([^\]]+)\]", db)[1]
        self.assertEqual(re.findall(r'"(\w+)"', device), ["sequent_os_initialize_device"])
        for check in ("m.member = current_user::regrole", "m.inherit_option", "m.admin_option",
                      "pg_has_role(current_user,r.oid,'MEMBER')", "r.rolname::text <> ALL($2::text[])"):
            self.assertIn(check, db)

    def test_polling_and_restore_cannot_register_or_approve(self):
        dialog = (PORTAL / "components/signing/OperationalApprovalDialog.tsx").read_text()
        observation = dialog.split("const read = useCallback", 1)[1].split("const run =", 1)[0]
        self.assertIn("localSetup.status(", observation)
        self.assertNotRegex(observation, r"\.register\(|\.start\(|\.setup\(")
        callback = (PORTAL / "lib/signing/finishOperationalApproval.ts").read_text()
        self.assertNotRegex(callback, r"SelfSetupRegister|localSetup\.register|console\.")
        ordered(self, callback, "readTabMarker(storage)", "clearTabMarker(storage)", 'tab.purpose === "initialize-setup"',
                "captured.issuer !== registration.issuer", "await silentNormalSession(", "await fetch(graph",
                "operationalInitializeSelfSetupConfirm", "bindReturnedPage(tab, returned", 'resume(returned.phase === "setup-confirmation-required"')
        self.assertEqual(dialog.count("await localSetup.register("), 1)
        self.assertIn("original.revision !== setupState?.revision", dialog)


class StoredPredicates(unittest.TestCase):
    """Exercise the actual 2210 scalar predicates with a disposable in-memory row."""
    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.create_function("clock_timestamp", 0, lambda: 10)
        self.db.execute("""CREATE TABLE attempt(id, bound, officer, portal_session_sha256, delivery_revision,
            released_at, abandoned_at, claimed_until, gold_until)""")
        self.db.execute("INSERT INTO attempt VALUES(1,2,3,'session',0,9,NULL,NULL,11)")
        body = BROWSER_SQL.split("CREATE FUNCTION felix3_initialize_setup.take_delivery(", 1)[1]
        where = body.split("WHERE ", 1)[1].split(";", 1)[0]
        for word in ("challenge", "b", "session_digest", "reviewed_revision"):
            where = re.sub(r"\b" + word + r"\b", ":" + word, where)
        self.take = "UPDATE attempt AS a SET delivery_revision = a.delivery_revision + 1 WHERE " + where.replace("e.officer", ":officer")
        self.args = dict(challenge=1, b=2, officer=3, session_digest="session", reviewed_revision=0)

    def test_duplicate_click_can_only_deliver_once_even_before_claim(self):
        self.assertEqual(self.db.execute(self.take, self.args).rowcount, 1)
        self.assertEqual(self.db.execute(self.take, self.args).rowcount, 0)
        self.assertEqual(self.db.execute(self.take, {**self.args, "reviewed_revision": 1}).rowcount, 1)
        self.db.execute("UPDATE attempt SET claimed_until=11")
        self.assertEqual(self.db.execute(self.take, {**self.args, "reviewed_revision": 2}).rowcount, 0)

    def test_revoked_expired_abandoned_or_different_attempt_is_never_delivered(self):
        for key, value in (("challenge", 5), ("b", 6), ("officer", 7), ("session_digest", "different"), ("reviewed_revision", 1)):
            self.assertEqual(self.db.execute(self.take, {**self.args, key: value}).rowcount, 0)
        for clause in ["gold_until=10", "abandoned_at=9", "released_at=NULL", "claimed_until=11"]:
            self.db.execute("SAVEPOINT case_start")
            self.db.execute("UPDATE attempt SET " + clause)
            self.assertEqual(self.db.execute(self.take, self.args).rowcount, 0)
            self.db.execute("ROLLBACK TO case_start")
            self.db.execute("RELEASE case_start")
        self.assertIn("FROM felix3_initialize_setup.eligible(b,true) e", BROWSER_SQL.split(".take_delivery(", 1)[1])

    def test_no_login_or_table_grant_to_portal_and_owned_return_is_required(self):
        self.assertNotRegex(BROWSER_SQL, r"(?i)CREATE (ROLE|USER)|GRANT[^;]*TO (PUBLIC|felix3_local_setup_portal)")
        read = BROWSER_SQL.split("CREATE FUNCTION felix3_initialize_setup.browser_attempt")[1].split("$$;")[0]
        for term in ("principal('felix3_local_setup_portal')", "eligible(b,false)", "(a.bound,a.officer) = (b,e.officer)",
                     "a.abandoned_at IS NULL", "a.portal_session_sha256 = session_digest"):
            self.assertIn(term, read)

    def test_original_2200_gate_requires_gold_release_then_a_single_unexpired_claim(self):
        claim = SETUP_SQL.split("CREATE FUNCTION felix3_initialize_setup.claim(", 1)[1].split("END;")[0]
        gate = claim.split("INTO authorized_until FROM felix3_initialize_setup.attempt a", 1)[1].split(" FOR UPDATE", 1)[0]
        gate = gate.replace("(challenge,b,permit.officer)", "(:challenge,:b,:officer)")
        count = lambda: self.db.execute("SELECT count(*) FROM attempt a " + gate, self.args).fetchone()[0]
        self.db.execute("UPDATE attempt SET released_at=NULL")
        self.assertEqual(count(), 0)
        self.db.execute("UPDATE attempt SET released_at=9")
        self.assertEqual(count(), 1)
        self.db.execute("UPDATE attempt SET claimed_until=11")
        self.assertEqual(count(), 0)
        ordered(self, claim, "eligible(b,true)", "FOR UPDATE OF a", "operational_setup_prepare_generation(",
                "operational_setup_claim_generation(", "SET claimed_until", "RETURN QUERY SELECT generation.fence")

    def test_2200_confirmation_keeps_signed_freshness_and_both_required_executions(self):
        sql = SETUP_SQL
        predicate = sql.split("CREATE FUNCTION felix3_initialize_setup.fresh_gold(")[1].split("$$")[1].strip()
        for name in ("opened", "authenticated", "issued", "expires", "observed"):
            predicate = re.sub(r"\b" + name + r"\b", ":" + name, predicate)
        times = dict(opened=10, authenticated=11, issued=12, expires=100, observed=13)
        self.assertEqual(self.db.execute(predicate, times).fetchone()[0], 1)
        for changed in [dict(authenticated=10), dict(observed=71), dict(issued=14), dict(expires=13)]:
            self.assertEqual(self.db.execute(predicate, {**times, **changed}).fetchone()[0], 0)
        confirmation = sql.split("CREATE FUNCTION felix3_initialize_setup.confirm_gold(")[1].split("END;")[0]
        for term in ("subject_id <> b.official", "felix3-local-initialize-setup", "a.exchange_at IS NOT NULL",
                     "a.auth_time IS NULL", "felix3_initialize_setup.fresh_gold("):
            self.assertIn(term, confirmation)
        verifier = (BEYOND / "operational_oidc.rs").read_text()
        for term in ("amr.len() == 2", "PASSWORD_EXECUTION", "OTP_EXECUTION", "proves_both_factors(&"):
            self.assertIn(term, verifier)


if __name__ == "__main__":
    unittest.main()
