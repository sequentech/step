# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline checks for the local setup handoff. Only scalar SELECTs are evaluated."""

from pathlib import Path
import re
import sqlite3
import unittest


ROOT = Path(__file__).resolve().parents[4]
MIGRATION = ROOT / "step/hasura/migrations/backend-db/1794700002200_local_initialize_self_setup"
SQL = (MIGRATION / "up.sql").read_text()
PORTAL = (ROOT / "step/packages/harvest/src/routes/local_initialize_self_setup.rs").read_text()
GOLD = (ROOT / "beyond/packages/hsm-service/src/felix3_local_initialize/setup_gold.rs").read_text()


def body(name):
    declaration = "CREATE FUNCTION felix3_initialize_setup." + name + "("
    return SQL.split(declaration, 1)[1].split("AS $$", 1)[1].split("$$;", 1)[0]


def ordered(case, source, *pieces):
    cursor = 0
    for piece in pieces:
        position = source.find(piece, cursor)
        case.assertGreaterEqual(position, 0, piece)
        cursor = position + len(piece)


class SetupBoundary(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.execute("PRAGMA query_only=ON")
        self.db.create_function("clock_timestamp", 0, lambda: 100)

    def accepted(self, predicate, **changes):
        # Use the literal WHERE clauses from 2200 over scalar inputs. One
        # opaque 'bound' represents the exact typed composite in PostgreSQL.
        values = dict(id="challenge", bound="request", officer="officer", nonce="nonce",
                      abandoned=None, exchanged=None, released=99, claimed=None, expires=120, gold=110,
                      expected_id="challenge", expected_bound="request",
                      expected_officer="officer", expected_nonce="nonce", permit_until=120)
        values.update(changes)
        query = """
            WITH a(id,bound,officer,nonce_sha256,abandoned_at,exchange_at,released_at,claimed_until,expires_at,gold_until) AS
              (VALUES (:id,:bound,:officer,:nonce,:abandoned,:exchanged,:released,:claimed,:expires,:gold)),
            permit(officer,until_at) AS (VALUES (:expected_officer,:permit_until)),
            input(challenge,b,nonce_hash,clicked,opened) AS (VALUES (:expected_id,:expected_bound,:expected_nonce,100,100))
            SELECT EXISTS(SELECT 1 FROM a,permit,input WHERE """ + predicate + ")"
        return bool(self.db.execute(query, values).fetchone()[0])

    def test_exact_code_exchange_cas(self):
        function = body("gold_challenge")
        update = function.split("RETURN QUERY UPDATE", 1)[1]
        where = update.split("WHERE", 1)[1].split("RETURNING", 1)[0]
        self.assertTrue(self.accepted(where))
        for change in [dict(id="other"), dict(bound="other"), dict(officer="other"), dict(nonce="other"),
                       dict(abandoned=90), dict(exchanged=99), dict(expires=100)]:
            with self.subTest(change=change):
                self.assertFalse(self.accepted(where, **change))

    def test_claim_requires_the_exact_ticket_and_the_post_gold_click(self):
        function = body("claim")
        where = function.split("INTO authorized_until FROM", 1)[1].split("WHERE", 1)[1].split("FOR UPDATE OF a", 1)[0]
        self.assertTrue(self.accepted(where))
        for change in [dict(id="other"), dict(bound="other"), dict(officer="other"), dict(abandoned=90),
                       dict(released=None), dict(claimed=110), dict(gold=None), dict(gold=100), dict(gold=99)]:
            with self.subTest(change=change):
                self.assertFalse(self.accepted(where, **change))
        ordered(self, function, "eligible(b,true)", "operational-signing-official:",
                "FOR UPDATE OF a", "operational_setup_prepare_generation(", "operational_setup_claim_generation(",
                "RAISE EXCEPTION", "RETURN QUERY SELECT generation.fence, least(authorized_until,generation.deadline)")

    def test_post_gold_click_requires_current_proof_and_permit(self):
        release = body("release").split("UPDATE felix3_initialize_setup.attempt", 1)[1]
        where = release.split("WHERE", 1)[1].split("AND NOT EXISTS", 1)[0]
        self.assertTrue(self.accepted(where, released=None))
        for change in [dict(gold=None), dict(gold=100), dict(permit_until=100), dict(officer="other"),
                       dict(bound="other"), dict(id="other"), dict(abandoned=90), dict(claimed=110)]:
            with self.subTest(change=change):
                self.assertFalse(self.accepted(where, **change))
        self.assertIn("coalesce(a.released_at,clicked)", release)
        self.assertIn("FROM sequent_backend.operational_setup_generation", release)

    def test_only_provably_unused_login_can_be_replaced(self):
        retire = body("begin_setup").split("SET abandoned_at = opened", 1)[1].split(";", 1)[0]
        where = retire[retire.index("a.abandoned_at IS NULL"):]
        self.assertTrue(self.accepted(where, released=None, exchanged=None, expires=100, gold=None))
        self.assertTrue(self.accepted(where, released=None, exchanged=80, expires=105, gold=100))
        self.assertFalse(self.accepted(where, released=None, exchanged=80, expires=90, gold=None))
        self.assertFalse(self.accepted(where, released=80, exchanged=79, expires=90, gold=89))

    def test_keycloak_numeric_dates(self):
        query = ("WITH t(opened,authenticated,issued,expires,observed) AS (VALUES (?,?,?,?,?)) "
                 + body("fresh_gold").strip().rstrip(";") + " FROM t")
        vectors = [
            ((40, 41, 42, 101, 100), True), ((40, 41, 41, 100, 99), True),
            ((40, 40, 42, 101, 50), False), ((40, 41, 40, 101, 50), False),
            ((40, 41, 51, 101, 50), False), ((40, 41, 42, 102, 101), False),
            ((40, 41, 42, 100, 100), False),
        ]
        for inputs, expected in vectors:
            with self.subTest(inputs=inputs):
                self.assertEqual(bool(self.db.execute(query, inputs).fetchone()[0]), expected)

    def test_principal_separation_and_rollback(self):
        entries = {"status": "portal", "begin_setup": "portal", "release": "portal",
                   "gold_challenge": "gold", "confirm_gold": "gold", "claim": "device", "complete": "device"}
        for name, role in entries.items():
            ordered(self, body(name), f"principal('felix3_local_setup_{role}')", "eligible(b,")
        for name in ["eligible", "status"]:
            self.assertNotRegex(body(name), r"(?im)^\s*(INSERT|UPDATE|DELETE)\s")
        self.assertNotRegex(SQL, r"(?im)^\s*(CREATE ROLE|GRANT .* TO (PUBLIC|felix3_local_setup_))")
        self.assertNotIn("operational_signing_demo_fixture_id", SQL)
        self.assertNotIn("ballot_publication_id", SQL)
        down = (MIGRATION / "down.sql").read_text()
        for name in re.findall(r"CREATE FUNCTION felix3_initialize_setup\.(\w+)\(", SQL):
            self.assertIn("felix3_initialize_setup." + name + "(", down)
        ordered(self, down, "ACCESS EXCLUSIVE MODE NOWAIT", "RAISE EXCEPTION", "DROP FUNCTION", "DROP SCHEMA")

    def test_actual_authorizer_and_server_owned_exchange(self):
        ordered(self, PORTAL, "caller(&claims.0)", "prepare_initialize_signing(", "pin.verify(",
                "current.payload_sha256() != reviewed_digest", "tx.rollback()")
        for name, operation in [("begin", "begin_setup"), ("release", "release"), ("status", "status")]:
            ordered(self, PORTAL.split("async fn " + name + "(", 1)[1].split("\n}", 1)[0],
                    "checked_scope(", "felix3_initialize_setup." + operation + "(")
        ordered(self, GOLD, "pub(crate) async fn confirm(", "validate_return(",
                "self.challenge(scope, attempt, &nonce, true)", "self.par.exchange(",
                "verify_with_iat(", "felix3_initialize_setup.confirm_gold(")
        for boundary in ("for_local_initialize(registration, secret)", "local_jwks(&install.pinned_jwks)",
                         "registered_approval_client == CLIENT", "registration.approval_client != CLIENT"):
            self.assertIn(boundary, GOLD)


if __name__ == "__main__":
    unittest.main()
