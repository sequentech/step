# SPDX-License-Identifier: AGPL-3.0-only
"""Offline wiring/authority contract. Not an end-to-end enrollment or SQL integration test."""
import re
import unittest
from pathlib import Path
import yaml

ROOT = Path(__file__).resolve().parents[3]
HERE = ROOT / "packages/harvest/src/routes"
LOCAL = ROOT / "hasura/felix3-local-initialize"
BRIDGE = (HERE / "local_officer_bridge.rs").read_text()
MOUNT = (HERE / "local_initialize_authority.rs").read_text()
SQL = (LOCAL / "1794700002260_officer_bridge/up.sql").read_text()
SCHEMA = (LOCAL / "actions.graphql").read_text()
ACTIONS = {r["name"]: r for r in yaml.safe_load((LOCAL/"actions.yaml").read_text())["actions"]}
ROUTES = {
    "operationalInitializeNomineePresence": ("presence","sign-initialize-voting",{"request_id","payload_sha256"}),
    "operationalInitializeOfficerPending": ("pending","signing-certificates-register",{"election_event_id"}),
    "operationalInitializeOfficerReview": ("review","signing-certificates-register",{"request_id","official_id","payload_sha256"}),
    "operationalInitializeOfficerAttach": ("attach","signing-certificates-register",{"request_id","official_id","payload_sha256","csr_sha256","leaf_sha256","leaf_der"}),
}
class Contract(unittest.TestCase):
    def test_all_client_actions_are_real_rocket_routes_with_exact_roles_and_closed_inputs(self):
        for name,(route,role,args) in ROUTES.items():
            action=ACTIONS[name]
            self.assertEqual(action["permissions"],[{"role":role}])
            self.assertTrue(action["definition"]["forward_client_headers"])
            self.assertEqual(action["definition"]["request_transform"]["body"]["template"],"{{$body.input}}")
            path="/operational-signing/initialize/attachment/"+route
            self.assertEqual(action["definition"]["handler"],"http://{{HARVEST_DOMAIN}}"+path)
            self.assertIn('"'+path+'"',BRIDGE)
            self.assertIn("attachment::"+route,MOUNT)
            declaration=re.search(name+r"\(([^)]*)\)",SCHEMA).group(1)
            self.assertEqual(set(re.findall(r"(\w+)\s*:",declaration)),args)
    def test_only_protected_mount_and_real_guards(self):
        self.assertIn("harvest/attachment/registration.json",MOUNT)
        self.assertIn("attachment::Bridge::load",MOUNT)
        self.assertRegex(MOUNT,r"Some\(bridge\)\s*=>\s*application\.manage\(bridge\)\.mount")
        normalized=re.sub(r"\s+","",BRIDGE)
        for token in ["fn presence(", "fn pending(", "fn review(", "fn attach("]:
            loc=normalized.index(token.replace(" ",""))
            self.assertIn("session:SetupSession",normalized[loc:loc+310])
        for required in ["gate.current_nominee","pin.verify","ctx.gate.review_csr","ctx.gate.register_and_attach",
                         "Permissions::SIGNING_CERTIFICATES_REGISTER","Permissions::GOLD","decode_permission_labels",
                         "system_user","peer:felix3_local_attachment_officer","single_public_leaf"]:
            self.assertIn(required,normalized)
        self.assertNotIn("getAccessToken",BRIDGE)
        self.assertNotIn("from_claims(",BRIDGE)
    def test_selectors_have_no_actor_or_issuance_bypass(self):
        self.assertNotRegex(SQL.upper(),r"\b(?:UPDATE|DELETE|INSERT)\s+")
        self.assertNotRegex(SQL.upper(),r"\bGRANT\b\s+(?:EXECUTE|SELECT|INSERT|UPDATE|ALL|USAGE)")
        self.assertNotIn("RETURNS boolean",SQL)
        for x in ["a.portal_session_sha256 = session_digest", "e.nonexportable_confirmed AND e.revoked_at IS NULL",
                  "p.officer <> b.official", "felix3_initialize_setup.eligible(b,false)",
                  "felix3_initialize_attachment.officer_service()", "felix3_initialize_attachment.generation",
                  "felix3_initialize_attachment.trust", "(SELECT count(*) FROM roots) = 1",
                  "(SELECT count(*) FROM candidate) = 1"]:
            self.assertIn(x,SQL)
        for returns in re.findall(r"RETURNS TABLE \(([^)]*)\)",SQL):
            self.assertNotRegex(returns,r"csr(?:_pem)?\s+text|root_pem|crl_der|boolean")

class ResolverRows(unittest.TestCase):
    """Execute the actual resolver JOIN/WHERE with inert rows, not a tenant or PKI."""
    def setUp(self):
        import sqlite3
        self.db=sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.hsm=True
        self.db.create_function("clock_timestamp",0,lambda:1000)
        self.db.create_function("octet_length",1,lambda p:None if p is None else len(p.encode()))
        self.db.create_function("hsm_required",1,lambda _:int(self.hsm))
        self.db.create_function("regexp",2,lambda pattern,s:bool(s and re.search(pattern,s)))
        self.db.executescript("""
          create table bindings(bound,tenant,event,official,request);
          create table permits(officer,until_at);
          create table attempts(id,bound,officer,portal_session_sha256,abandoned_at,released_at,claimed_until);
          create table generations(attempt,tenant_id,event_id,official,prepared_by,device_profile,spki_sha256,public_csr_pem,step,fence,evidence_sha256,enrollment_id);
          create table enrollments(id,tenant_id,actor,spki,device_profile,nonexportable_confirmed,revoked_at);
          create table approvals(tenant_id,election_event_id,request_id,user_id);
          insert into bindings values ('pinned-publication','tenant','event','nominee','request');
          insert into permits values('officer',1200);
          insert into attempts values('attempt','pinned-publication','officer','original-session',null,900,920);
        """)
        spki='a'*64
        self.db.execute("insert into generations values(?,?,?,?,?,?,?,?,?,?,?,?)",
            ("attempt","tenant","event","nominee","officer","softhsm-test",spki,"only-public-csr","completed","fence","evidence","enrollment"))
        self.db.execute("insert into enrollments values(?,?,?,?,?,?,?)",
            ("enrollment","tenant","nominee",spki,"softhsm-test",1,None))
        body=re.search(r"FROM felix3_initialize_setup\.eligible\(b,false\) p(.*?);",SQL,re.S).group(1)
        for old,new in (
            ("felix3_initialize_setup.attempt","attempts"),
            ("sequent_backend.operational_setup_generation","generations"),
            ("sequent_backend.operational_signing_official_enrollment","enrollments"),
            ("sequent_backend.operational_signing_hsm_required","hsm_required"),
            ("sequent_backend.signing_approval","approvals"),
            ("SELECT FROM approvals","SELECT 1 FROM approvals"),
            ("(b,p.officer)","(b.bound,p.officer)"),
            ("b.official::text","b.official"),
            ("= session_digest","= :sid"),
            ("g.spki_sha256 ~ '^[0-9a-f]{64}$'","regexp('^[0-9a-f]{64}$',g.spki_sha256)"),
        ):
            self.assertIn(old,body)
            body=body.replace(old,new)
        self.query="SELECT a.id,g.enrollment_id FROM bindings b CROSS JOIN permits p "+body
    def result(self,session="original-session"):
        return self.db.execute(self.query,{"sid":session}).fetchall()
    def test_exact_committed_generation_only(self):
        self.assertEqual(self.result(),[("attempt","enrollment")])
        self.assertEqual(self.result("another-session"),[])
        for change in (
            "update attempts set bound='different-publication'",
            "update attempts set abandoned_at=991",
            "update attempts set released_at=null",
            "update attempts set claimed_until=null",
            "update permits set officer='somebody-else'",
            "update permits set until_at=1000",
            "update bindings set official='officer'",
            "update generations set device_profile='yubihsm'",
            "update generations set step='prepared'",
            "update generations set fence=null",
            "update generations set public_csr_pem=''",
            "update generations set evidence_sha256=null",
            "update generations set prepared_by='unassigned-officer'",
            "update generations set event_id='different-event'",
            "update enrollments set nonexportable_confirmed=0",
            "update enrollments set revoked_at=999",
            "update enrollments set spki='substitution'",
            "insert into approvals values('tenant','event','request','nominee')",
        ):
            with self.subTest(change=change):
                self.db.execute("SAVEPOINT mutation")
                self.db.execute(change)
                self.assertEqual(self.result(),[])
                self.db.execute("ROLLBACK TO mutation")
                self.db.execute("RELEASE mutation")
        self.hsm=False
        self.assertEqual(self.result(),[])

if __name__ == "__main__":
    unittest.main()
