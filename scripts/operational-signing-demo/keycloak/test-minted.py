#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Build a private synthetic import or exercise a dedicated loopback Keycloak."""
import base64
import hashlib
import hmac
from http.cookiejar import CookieJar
from html.parser import HTMLParser
import importlib.util
import json
import os
from pathlib import Path
import secrets
import ssl
import struct
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path("/fixture")
TENANT = uuid.UUID("98f5275b-e4f9-424a-83d4-0c67021d69ef")
ORIGIN = "https://127.0.0.1:18443"
OIDC = f"{ORIGIN}/realms/tenant-{TENANT}/protocol/openid-connect"
PORTAL = "https://127.0.0.1:13443/"
APPROVAL = PORTAL + "operational-signing/return"
SESSION_RETURN = PORTAL + "silent-check-sso.html"
ISSUER_HOST = "127.0.0.1:18443"
LOGIN_PATH = f"/realms/tenant-{TENANT}/login-actions/authenticate"


def issuer_url(url):
    parsed = urllib.parse.urlsplit(url)
    return (
        parsed.scheme == "https"
        and parsed.netloc == ISSUER_HOST
        and not parsed.fragment
    )


def matching_callback(url, expected):
    if expected not in {PORTAL, APPROVAL, SESSION_RETURN}:
        return False
    actual = urllib.parse.urlsplit(url)
    target = urllib.parse.urlsplit(expected)
    return (actual.scheme, actual.netloc, actual.path) == (
        target.scheme, target.netloc, target.path
    )

def b64(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode()

def prepare(root):
    # These are throwaway fixture passwords and client secrets. Never use
    # a shared directory, an earlier fixture, or an ordinary provider.
    if root.is_symlink() or not root.is_dir() or root.stat().st_mode & 0o077 or any(root.iterdir()):
        raise SystemExit("prepare needs an empty private directory (mode 0700)")
    path = Path(__file__).with_name("render-realm.py")
    spec = importlib.util.spec_from_file_location("render", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    secret = secrets.token_urlsafe(42)
    realm = mod.render(TENANT, secret)
    account = dict(secret=secret, users=[])
    realm["users"] = []
    for index in range(3):
        user = f"fixture-approver-{index}"
        password = secrets.token_urlsafe(30)
        subject = str(uuid.uuid4())
        otp = base64.b32encode(secrets.token_bytes(20)).decode()
        credentials = [
            {"type": "password", "value": password, "temporary": False},
            {
                "type": "otp",
                "secretData": json.dumps({"value": otp}),
                "credentialData": json.dumps(
                    {
                        "subType": "totp",
                        "digits": 6,
                        "counter": 0,
                        "period": 30,
                        "algorithm": "HmacSHA1",
                        "secretEncoding": "BASE32",
                    }
                ),
            },
        ]
        claim = {
            "x-hasura-default-role": "sign-initialize-voting",
            "x-hasura-user-id": subject,
            "x-hasura-tenant-id": str(TENANT),
            "x-hasura-allowed-roles": ["sign-initialize-voting"],
        }
        realm["users"].append(
            {
                "id": subject,
                "username": user,
                "enabled": True,
                "firstName": "Fixture",
                "lastName": str(index),
                "email": user + "@example.invalid",
                "emailVerified": True,
                "credentials": credentials,
                "attributes": {"fixture-14990-hasura-claims": [json.dumps(claim)]},
            }
        )
        account["users"].append(
            {"username": user, "password": password, "sub": subject, "otp": otp}
        )
    with (root / "realm.json").open("x") as f:
        json.dump(realm,f)
    with (root / "accounts.json").open("x") as f:
        json.dump(account,f)

class Form(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.forms = []
        self.active = None
    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "form" and a.get("method","").lower() == "post" and "action" in a:
            self.active = (a.get("id") or "", a["action"], {}, set())
            self.forms.append(self.active)
        if tag == "input" and self.active and a.get("name"):
            self.active[3].add(a["name"])
            if a.get("type") == "hidden":
                self.active[2][a["name"]] = a.get("value","")
    def handle_endtag(self, tag):
        if tag == "form":
            self.active = None

def classify_form(html):
    """Only controlled labels, never tokens, URLs, field names, or values."""
    parser = Form()
    parser.feed(html)
    if any("otp" in form[3] for form in parser.forms):
        return "OTP form"
    if any("password" in form[3] for form in parser.forms):
        return "password form"
    return "other response"


class LocalRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # urllib must never forward a submitted form or Basic auth header.
        # Only the issuer's GET navigation is followed; Portal returns are
        # inspected locally without making a request to the callback.
        if issuer_url(newurl) and code in (301, 302, 303):
            return urllib.request.Request(newurl)
        return None


def totp(secret):
    counter = struct.pack(">Q", int(time.time()) // 30)
    signature = hmac.new(base64.b32decode(secret), counter, hashlib.sha1).digest()
    offset = signature[-1] & 15
    digits = int.from_bytes(signature[offset : offset + 4], "big") & 0x7FFFFFFF
    return f"{digits % 10**6:06d}"

def unb64(s):
    return base64.urlsafe_b64decode(s + "===")

def claims(jwt, jwks):
    header, payload, signature = jwt.split(".")
    decoded = json.loads(unb64(header))
    key = next(k for k in jwks if k.get("kid") == decoded["kid"])
    assert decoded["alg"] == "RS256" and key["kty"] == "RSA", "wrong token signer"
    n, e = (int.from_bytes(unb64(key[f]), "big") for f in ("n", "e"))
    signed = int.from_bytes(unb64(signature), "big")
    width = (n.bit_length() + 7) // 8
    digest_info = bytes.fromhex("3031300d060960864801650304020105000420") + hashlib.sha256(
        (header + "." + payload).encode()).digest()
    expected = b"\0\1" + b"\xff" * (width - len(digest_info) - 3) + b"\0" + digest_info
    assert signed < n and pow(signed, e, n).to_bytes(width, "big") == expected, "fixture JWT signature invalid"
    return json.loads(unb64(payload))

def run(root):
    fixtures = json.loads((root / "accounts.json").read_text())
    ctx = ssl.create_default_context(cafile=str(root / "tls.crt"))

    def new_browser():
        return urllib.request.build_opener(
            urllib.request.ProxyHandler({}),
            LocalRedirect(),
            urllib.request.HTTPCookieProcessor(CookieJar()),
            urllib.request.HTTPSHandler(context=ctx),
        )

    def request(opener, url, data=None, basic=None):
        assert issuer_url(url), "request must stay on the exact fixture issuer"
        try:
            encoded = None if data is None else urllib.parse.urlencode(data).encode()
            req = urllib.request.Request(url, encoded)
            if basic:
                auth = f"{basic}:{fixtures['secret']}".encode()
                req.add_header("Authorization", "Basic " + base64.b64encode(auth).decode())
            r = opener.open(req, timeout=8)
        except urllib.error.HTTPError as e:
            r = e
        with r:
            return r.code, r.headers.get("Location"), r.read().decode()

    code, _, body = request(new_browser(), OIDC + "/certs")
    assert code == 200, "Keycloak fixture did not expose its signed token verifier"
    jwks = json.loads(body)["keys"]
    def login(client, redirect, customer, *, gold=False, par=False, silent=False, opener=None):
        user = fixtures["users"][customer]
        opener = opener or new_browser()
        verifier = secrets.token_urlsafe(45)
        p = {
            "client_id": client,
            "redirect_uri": redirect,
            "response_type": "code",
            "scope": "openid",
            "state": secrets.token_urlsafe(20),
            "nonce": secrets.token_urlsafe(20),
            "code_challenge_method": "S256",
            "code_challenge": b64(hashlib.sha256(verifier.encode()).digest()),
        }
        if silent:
            p["prompt"] = "none"
        if gold:
            required = {"acr": {"essential": True, "values": ["gold"]}}
            if par:
                required["amr"] = {"essential": True}
                p.update(
                    max_age="0",
                    response_mode="fragment",
                    state=secrets.token_urlsafe(32),
                    nonce=secrets.token_urlsafe(32),
                    acr_values="gold",
                )
            p.update(prompt="login", claims=json.dumps({"id_token": required}))
        auth = p
        if par:
            code, _, body = request(opener, OIDC + "/ext/par/request", p, basic=client)
            assert code == 201, f"PAR POST status {code}"
            auth = {"client_id": client, "request_uri": json.loads(body)["request_uri"]}
        code, location, body = request(
            opener, OIDC + "/auth?" + urllib.parse.urlencode(auth)
        )

        def post(status, text, values, step):
            parser = Form()
            parser.feed(text)
            matches = [form for form in parser.forms if values.keys() & form[3]]
            assert status == 200 and len(matches) == 1, (
                f"{step} form unavailable ({status}): {classify_form(text)}"
            )
            _, action, hidden, _ = matches[0]
            assert issuer_url(action), "login form issuer mismatch"
            assert urllib.parse.urlsplit(action).path == LOGIN_PATH, (
                "login form path mismatch"
            )
            selected = {}
            if step == "OTP" and "selectedCredentialId" in hidden:
                selected["selectedCredentialId"] = hidden["selectedCredentialId"]
            return request(opener, action, selected | values)

        if not silent:
            credentials = {"username": user["username"], "password": user["password"]}
            code, location, body = post(code, body, credentials, "password")
        if gold:
            assert code == 200, "password-only reached a redirect"
            code, location, body = post(code, body, {"otp": totp(user["otp"])}, "OTP")

        assert code in (302, 303) and location and matching_callback(location, redirect), (
            f"callback refused ({code}): {classify_form(body)}"
        )
        returned = urllib.parse.urlsplit(location)
        assert not (returned.query if par else returned.fragment), "callback component mismatch"
        query = urllib.parse.parse_qs(returned.fragment if par else returned.query)
        assert query.get("state") == [p["state"]], "callback state mismatch"
        assert len(query.get("code", [])) == 1 and "error" not in query, (
            "callback lacked a single authorization code"
        )
        form = {
            "code": query["code"][0],
            "grant_type": "authorization_code",
            "client_id": client,
            "code_verifier": verifier,
            "redirect_uri": redirect,
        }
        code, _, body = request(opener, OIDC + "/token", form, basic=client if par else None)
        assert code == 200, f"token endpoint status {code}"
        tokens = json.loads(body)
        c = claims(tokens["id_token" if par else "access_token"], jwks)
        assert c["sub"] == user["sub"] and c["iss"] == OIDC.split("/protocol/")[0], (
            "unexpected actor or realm"
        )
        if par:
            assert (
                c["nonce"] == p["nonce"]
                and c["azp"] == client
                and c["typ"] == "ID"
                and c["aud"] in (client, [client])
            ), "PAR ceremony token not exact"
        return c, opener
    failures = []
    for key, customer, client, return_url, gold, par in (
        ("normal audience and actor", 0, "admin-portal", PORTAL, False, False),
        ("fresh Admin setup Gold", 1, "admin-portal", PORTAL, True, False),
        ("dedicated approval Gold and PAR", 2, "sequent-operational-demo-14990", APPROVAL, True, True),
    ):
        try:
            c, browser = login(client, return_url, customer, gold=gold, par=par)
            if not gold:
                assert c["azp"]=="admin-portal" and "admin-portal" in c["aud"], "ordinary audience absent"
                assert c["acr"]!="gold", "ordinary password alone obtained Gold"
                assert c["https://hasura.io/jwt/claims"]["x-hasura-user-id"]==c["sub"], "actor claim missing/mismatch"
            else:
                assert c["acr"]=="gold" and 0 <= time.time()-c.get("auth_time",0) < 60, "fresh Gold missing"
                if par:
                    assert set(c.get("amr",[]))=={"sequent-op-pwd","sequent-op-otp"}, "approval signed AMR not exact"
                    resumed, _ = login("admin-portal", SESSION_RETURN, customer,
                                       silent=True, opener=browser)
                    assert resumed["azp"]=="admin-portal" and "admin-portal" in resumed["aud"], "wrong Portal callback audience"
                    assert resumed["sub"] == c["sub"], "Portal callback restored a wrong actor"
                    print("PASS Portal silent callback restored the same issuer subject", flush=True)
            print(f"PASS {key}", flush=True)
        except (AssertionError, KeyError, ValueError, StopIteration, OSError, TypeError):
            print(f"FAIL {key} (response details withheld)", flush=True)
            failures.append(key)
    if failures:
        raise SystemExit(1)

def main():
    os.umask(0o077)
    sys.dont_write_bytecode = True
    if not __debug__:
        raise SystemExit("This fixture must run with assertions enabled.")
    if len(sys.argv) == 3 and sys.argv[1] == "prepare":
        prepare(Path(sys.argv[2]))
    elif len(sys.argv) == 2 and sys.argv[1] == "run":
        try:
            run(ROOT)
        except (AssertionError, KeyError, ValueError, StopIteration, OSError, TypeError):
            raise SystemExit("Fixture could not start its checks (details withheld).") from None
    else:
        raise SystemExit(2)


if __name__ == "__main__":
    main()
