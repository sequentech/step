#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Regroup a closed election event's voters into areas named after one of
their attributes, moving their existing cast votes with them, so that a new
tally reports one result per attribute value.

Usage and the full procedure are in
docs/docusaurus/docs/09-support/02-regroup-voters-by-attribute.md.
Only the Python standard library is used.
"""

import argparse
import enum
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from collections import Counter, defaultdict

AREA_ID_ATTRIBUTE = "area-id"
VOTE_WEIGHT_ATTRIBUTE = "vote-weight"
TOOL_ANNOTATION = "regrouped_by_attribute"
IN_PROGRESS = "in-progress"
VALID = "valid"
TERMINAL_TALLY_STATUSES = {"SUCCESS", "CANCELLED"}
ONLINE_CHANNEL = "voting_status"
OTHER_CHANNELS = ("early_voting_status", "kiosk_voting_status", "telephone_voting_status")
CLOSED = "CLOSED"
NOT_STARTED = "NOT_STARTED"
# Same threshold rc.4 windmill warns about for weighted batches.
MIN_ANONYMITY_VOTERS = 5
PAGE_SIZE = 500
KEYCLOAK_PAGE_SIZE = 100
# Rows existing before the change must keep these hashes afterwards. The
# excluded columns are the ones this procedure, or the republication it needs,
# legitimately changes.
FINGERPRINTED_TABLES = {
    "tally_session": (),
    "tally_session_contest": (),
    "tally_session_execution": (),
    "results_event": (),
    "results_election": (),
    "results_election_area": (),
    "results_contest": (),
    "results_area_contest": (),
    "results_contest_candidate": (),
    "results_area_contest_candidate": (),
    "ballot_publication": ("deleted_at",),
    "ballot_style": ("deleted_at", "last_updated_at"),
    "cast_vote": ("area_id", "last_updated_at"),
}
EXIT_PUBLICATION_PENDING = 3


class BlankPolicy(enum.Enum):
    FAIL = "fail"
    KEEP_IN_SOURCE_AREA = "keep-in-source-area"


class RegroupError(Exception):
    def __init__(self, problems):
        self.problems = problems if isinstance(problems, list) else [problems]
        super().__init__("; ".join(self.problems))


def parse_uuid(value, name):
    try:
        parsed = str(uuid.UUID(value))
    except (ValueError, TypeError, AttributeError):
        parsed = None
    if parsed is None or parsed != value.lower():
        raise RegroupError(f"{name} is not a UUID: {value!r}")
    return parsed


def event_realm(tenant_id, election_event_id):
    return f"tenant-{tenant_id}-event-{election_event_id}"


def canonical_digest(value):
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def single_attribute(user, name):
    """The attribute's only value, None when absent; several values are an error."""
    values = (user.get("attributes") or {}).get(name) or []
    if len(values) > 1:
        raise RegroupError(f"user {user['id']} has {len(values)} values for {name}")
    return values[0] if values else None


def vote_weight(user):
    value = single_attribute(user, VOTE_WEIGHT_ATTRIBUTE)
    if value is None or value == "":
        return 1
    if not value.isdigit():
        raise RegroupError(f"user {user['id']} has a non-numeric {VOTE_WEIGHT_ATTRIBUTE}")
    return int(value)


def live_style_tuples(rows):
    """(election, area, contest) of every contest frozen in the given ballot styles."""
    tuples = set()
    for row in rows:
        style = json.loads(row["ballot_eml"] or "null")
        if not style:
            raise RegroupError(f"published ballot style {row['id']} has no ballot EML")
        for contest in style.get("contests") or []:
            tuples.add((style["election_id"], style["area_id"], contest["id"]))
    return sorted(tuples)


class Hasura:
    """The Hasura queries and the single mutation this procedure needs."""

    def __init__(self, endpoint, admin_secret, source, urlopen=urllib.request.urlopen):
        self.endpoint = endpoint.rstrip("/")
        self.admin_secret = admin_secret
        self.source = source
        self.urlopen = urlopen

    def _post(self, path, payload):
        request = urllib.request.Request(
            self.endpoint + path,
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json", "x-hasura-admin-secret": self.admin_secret},
            method="POST",
        )
        with self.urlopen(request) as response:
            return json.loads(response.read())

    def graphql(self, query, variables):
        body = self._post("/v1/graphql", {"query": query, "variables": variables})
        if body.get("errors"):
            raise RegroupError(f"Hasura error: {body['errors']}")
        return body["data"]

    def _paged(self, table, fields, scope):
        rows = []
        while True:
            data = self.graphql(
                f"query($w: sequent_backend_{table}_bool_exp!, $o: Int!, $l: Int!) {{"
                f" rows: sequent_backend_{table}(where: $w, order_by: {{id: asc}}, offset: $o, limit: $l)"
                f" {{ {fields} }} }}",
                {"w": scope, "o": len(rows), "l": PAGE_SIZE},
            )
            rows.extend(data["rows"])
            if len(data["rows"]) < PAGE_SIZE:
                return rows

    def load(self, tenant_id, election_event_id):
        scope = {"tenant_id": {"_eq": tenant_id}, "election_event_id": {"_eq": election_event_id}}
        event = self.graphql(
            "query($t: uuid!, $e: uuid!) { rows: sequent_backend_election_event("
            "where: {tenant_id: {_eq: $t}, id: {_eq: $e}}) { id status } }",
            {"t": tenant_id, "e": election_event_id},
        )["rows"]
        live = [
            row["id"]
            for row in self._paged("ballot_publication", "id published_at deleted_at", scope)
            if row["published_at"] and not row["deleted_at"]
        ]
        return {
            "event": event[0] if event else None,
            "elections": self._paged("election", "id status", scope),
            "areas": self._paged(
                "area", "id name type parent_id presentation annotations", scope
            ),
            "area_contests": self._paged("area_contest", "id area_id contest_id", scope),
            "cast_votes": self._paged(
                "cast_vote", "id voter_id_string area_id election_id status", scope
            ),
            "contests": self._paged("contest", "id election_id", scope),
            "tally_sessions": self._paged("tally_session", "id execution_status", scope),
            "live_styles": live_style_tuples(
                row for row in self._paged(
                    "ballot_style", "id ballot_eml ballot_publication_id deleted_at", scope
                )
                if not row["deleted_at"] and row["ballot_publication_id"] in live
            ),
        }

    def fingerprints(self, tenant_id, election_event_id):
        """Per-row SHA-256 of every fingerprinted table, read-only."""
        tenant_id = parse_uuid(tenant_id, "tenant id")
        election_event_id = parse_uuid(election_event_id, "election event id")
        result = {}
        for table, excluded in FINGERPRINTED_TABLES.items():
            minus = "".join(f" - '{column}'" for column in excluded)
            sql = (
                f"SELECT id::text, encode(sha256(convert_to((to_jsonb(t){minus})::text, 'UTF8')), 'hex')"
                f" FROM sequent_backend.{table} t WHERE tenant_id = '{tenant_id}'"
                f" AND election_event_id = '{election_event_id}' ORDER BY id"
            )
            body = self._post(
                "/v2/query",
                {"type": "run_sql", "args": {"source": self.source, "sql": sql, "read_only": True}},
            )
            if "result" not in body:
                raise RegroupError(f"Hasura run_sql error on {table}: {body}")
            result[table] = dict(body["result"][1:])
        return result

    def apply(self, tenant_id, election_event_id, source_area_id, areas, area_contests, moves):
        """Inserts and guarded moves in one request, which Hasura runs as one transaction.

        Only the variables a request uses are declared, since GraphQL rejects
        unused ones; a request with nothing to do is not sent.
        """
        declarations, fields, variables = [], [], {}
        if areas:
            declarations.append("$areas: [sequent_backend_area_insert_input!]!")
            fields.append("areas: insert_sequent_backend_area(objects: $areas) { affected_rows }")
            variables["areas"] = areas
        if area_contests:
            declarations.append("$contests: [sequent_backend_area_contest_insert_input!]!")
            fields.append(
                "contests: insert_sequent_backend_area_contest(objects: $contests) { affected_rows }"
            )
            variables["contests"] = area_contests
        ordered = sorted((target, ids) for target, ids in moves.items() if ids)
        if ordered:
            declarations += ["$t: uuid!", "$e: uuid!", "$s: uuid!"]
            variables.update({"t": tenant_id, "e": election_event_id, "s": source_area_id})
        for index, (target, ids) in enumerate(ordered):
            declarations += [f"$ids{index}: [uuid!]!", f"$to{index}: uuid!"]
            fields.append(
                f"m{index}: update_sequent_backend_cast_vote(where: {{tenant_id: {{_eq: $t}},"
                f" election_event_id: {{_eq: $e}}, area_id: {{_eq: $s}}, id: {{_in: $ids{index}}}}},"
                f" _set: {{area_id: $to{index}}}) {{ affected_rows }}"
            )
            variables[f"ids{index}"] = ids
            variables[f"to{index}"] = target
        if not fields:
            return 0, 0, {target: 0 for target in moves}
        data = self.graphql(
            f"mutation({', '.join(declarations)}) {{ {' '.join(fields)} }}", variables
        )
        moved = {target: 0 for target in moves}
        moved.update({target: data[f"m{index}"]["affected_rows"]
                      for index, (target, _) in enumerate(ordered)})
        return (data.get("areas", {}).get("affected_rows", 0),
                data.get("contests", {}).get("affected_rows", 0), moved)


class Keycloak:
    """Keycloak admin REST access with a token refreshed before it expires."""

    def __init__(self, url, admin_realm, credentials, urlopen=urllib.request.urlopen):
        self.url = url.rstrip("/")
        self.admin_realm = admin_realm
        self.credentials = credentials
        self.urlopen = urlopen
        self.token = None
        self.expires_at = 0.0

    def _token(self):
        if self.token and time.monotonic() < self.expires_at - 10:
            return self.token
        request = urllib.request.Request(
            f"{self.url}/realms/{urllib.parse.quote(self.admin_realm)}/protocol/openid-connect/token",
            data=urllib.parse.urlencode(self.credentials).encode(),
            method="POST",
        )
        with self.urlopen(request) as response:
            body = json.loads(response.read())
        self.token = body["access_token"]
        self.expires_at = time.monotonic() + body.get("expires_in", 60)
        return self.token

    def _request(self, method, path, body=None):
        request = urllib.request.Request(
            f"{self.url}/admin/realms/{path}",
            data=None if body is None else json.dumps(body).encode(),
            headers={"Authorization": f"Bearer {self._token()}", "Content-Type": "application/json"},
            method=method,
        )
        with self.urlopen(request) as response:
            content = response.read()
            return json.loads(content) if content else None

    def list_users(self, realm):
        users = []
        while True:
            page = self._request(
                "GET",
                f"{urllib.parse.quote(realm)}/users?briefRepresentation=false"
                f"&first={len(users)}&max={KEYCLOAK_PAGE_SIZE}",
            )
            users.extend(page)
            if len(page) < KEYCLOAK_PAGE_SIZE:
                return [user for user in users if not user.get("serviceAccountClientId")]

    def get_user(self, realm, user_id):
        return self._request("GET", f"{urllib.parse.quote(realm)}/users/{user_id}")

    def put_user(self, realm, user):
        self._request("PUT", f"{urllib.parse.quote(realm)}/users/{user['id']}", user)


def check_closed(status, what):
    status = status or {}
    problems = []
    if status.get(ONLINE_CHANNEL) != CLOSED:
        problems.append(f"{what} online voting is {status.get(ONLINE_CHANNEL)!r}, not {CLOSED}")
    for channel in OTHER_CHANNELS:
        if status.get(channel) not in (None, NOT_STARTED, CLOSED):
            problems.append(f"{what} {channel} is {status.get(channel)!r}")
    return problems


def early_voting_presentation(area):
    """The source area's early voting policy, and nothing that names or describes it."""
    policy = (area.get("presentation") or {}).get("allow_early_voting")
    return None if policy is None else {"allow_early_voting": policy}


def voter_invariants(users, user_ids):
    """Digest of everything about each voter that regrouping must not change."""
    invariants = {}
    for user in users:
        if user["id"] in user_ids:
            attributes = {k: v for k, v in (user.get("attributes") or {}).items()
                          if k != AREA_ID_ATTRIBUTE}
            invariants[user["id"]] = canonical_digest({
                "enabled": user.get("enabled"), "username": user.get("username"),
                "attributes": attributes})
    return invariants


def operational_problems(state):
    """Voting closed everywhere, no tally running and no vote still being processed."""
    problems = check_closed(state["event"].get("status"), "event")
    for election in state["elections"]:
        problems += check_closed(election.get("status"), f"election {election['id']}")
    running = [s["id"] for s in state["tally_sessions"]
               if s["execution_status"] not in TERMINAL_TALLY_STATUSES]
    if running:
        problems.append(f"tally sessions not finished: {', '.join(running)}")
    unresolved = sum(1 for vote in state["cast_votes"] if vote["status"] == IN_PROGRESS)
    if unresolved:
        problems.append(f"{unresolved} cast vote(s) are still {IN_PROGRESS}")
    return problems


def build_plan(state, users, tenant_id, election_event_id, source_area_id, attribute,
               blank_policy, new_area_ids=None):
    """Validate the event and compute every change. Pure: reads only its inputs.

    `new_area_ids` maps area names to the ids chosen by an earlier plan, so that
    apply and verify recompute exactly the plan that was reviewed.
    """
    new_area_ids = dict(new_area_ids or {})
    if not state["event"]:
        raise RegroupError(f"election event {election_event_id} not found in tenant {tenant_id}")
    problems = operational_problems(state)

    areas = {area["id"]: area for area in state["areas"]}
    source = areas.get(source_area_id)
    if not source:
        raise RegroupError(problems + [f"source area {source_area_id} is not in this event"])
    marker = {"source_area_id": source_area_id, "attribute": attribute}
    tool_areas = {a["id"]: a for a in state["areas"]
                  if (a.get("annotations") or {}).get(TOOL_ANNOTATION) == marker}
    source_contests = {ac["contest_id"] for ac in state["area_contests"] if ac["area_id"] == source_area_id}
    election_of = {contest["id"]: contest["election_id"] for contest in state["contests"]}
    problems += [f"contest {c} of the source area is not in this event"
                 for c in sorted(source_contests - set(election_of))]
    if not source_contests:
        problems.append("source area has no contests")

    in_scope = {}
    blank_users = []
    out_of_scope = 0
    for user in users:
        try:
            current = single_attribute(user, AREA_ID_ATTRIBUTE)
            if current != source_area_id and current not in tool_areas:
                out_of_scope += 1
                continue
            value = single_attribute(user, attribute)
        except RegroupError as error:
            problems += error.problems
            continue
        if value is None or not value.strip():
            blank_users.append(user["id"])
            if blank_policy == BlankPolicy.FAIL:
                continue
            value = None
        elif value != value.strip():
            problems.append(f"user {user['id']} has leading or trailing whitespace in {attribute}")
            continue
        in_scope[user["id"]] = (user, current, value)
    if blank_users and blank_policy == BlankPolicy.FAIL:
        problems.append(
            f"{len(blank_users)} voter(s) have no {attribute}; fix the census or choose "
            f"--blank-policy {BlankPolicy.KEEP_IN_SOURCE_AREA.value} explicitly"
        )

    names = sorted({value for _, _, value in in_scope.values() if value is not None})
    folded = Counter(name.casefold() for name in names)
    problems += [f"values differ only by case: {key!r}" for key, count in folded.items() if count > 1]
    target_ids = {}
    areas_to_create = []
    for name in names:
        reused = [a for a in tool_areas.values() if a["name"] == name]
        foreign = [a for a in areas.values() if a["id"] not in tool_areas
                   and (a["name"] or "").casefold() == name.casefold()]
        if foreign:
            problems.append(f"area name {name!r} is already used by area {foreign[0]['id']}")
        elif len(reused) > 1:
            problems.append(f"several areas named {name!r} were created for this regrouping")
        elif reused:
            target_ids[name] = reused[0]["id"]
        else:
            area_id = new_area_ids.get(name) or str(uuid.uuid4())
            target_ids[name] = area_id
            areas_to_create.append({
                "id": area_id,
                "tenant_id": tenant_id,
                "election_event_id": election_event_id,
                "name": name,
                "type": source.get("type"),
                "parent_id": source.get("parent_id"),
                "presentation": early_voting_presentation(source),
                "annotations": {TOOL_ANNOTATION: marker},
            })

    contests_to_create = []
    for name, area_id in sorted(target_ids.items()):
        existing = {ac["contest_id"] for ac in state["area_contests"] if ac["area_id"] == area_id}
        if existing - source_contests:
            problems.append(f"area {name!r} has contests the source area does not have")
        for contest_id in sorted(source_contests - existing):
            contests_to_create.append({
                "id": str(uuid.uuid5(uuid.UUID(area_id), contest_id)),
                "tenant_id": tenant_id,
                "election_event_id": election_event_id,
                "area_id": area_id,
                "contest_id": contest_id,
            })

    target_of = {uid: (source_area_id if value is None else target_ids.get(value))
                 for uid, (_, _, value) in in_scope.items()}
    user_moves = [
        {"user_id": uid, "from": current, "to": target_of[uid]}
        for uid, (_, current, _) in sorted(in_scope.items())
        if target_of[uid] and current != target_of[uid]
    ]
    for move in user_moves:
        if move["from"] != source_area_id:
            problems.append(f"user {move['user_id']} is in another regrouped area; its {attribute} changed")

    scoped_areas = {source_area_id, *tool_areas}
    vote_moves = defaultdict(list)
    for vote in state["cast_votes"]:
        target = target_of.get(vote["voter_id_string"])
        if target is None:
            if vote["area_id"] in scoped_areas and vote["voter_id_string"] not in blank_users:
                problems.append(f"cast vote {vote['id']} belongs to no voter of the regrouped areas")
            continue
        if vote["area_id"] == target:
            continue
        if vote["area_id"] != source_area_id:
            problems.append(f"cast vote {vote['id']} is in area {vote['area_id']}, not the source area")
            continue
        vote_moves[target].append(vote["id"])

    if problems:
        raise RegroupError(problems)

    summary = defaultdict(lambda: {"voters": 0, "voted": 0, "voted_weight": 0, "cast_votes": 0})
    voted = {v["voter_id_string"] for v in state["cast_votes"] if v["status"] == VALID}
    name_of = {area_id: name for name, area_id in target_ids.items()}
    name_of[source_area_id] = source["name"]
    for uid, (user, _, _) in in_scope.items():
        entry = summary[name_of[target_of[uid]]]
        entry["voters"] += 1
        if uid in voted:
            entry["voted"] += 1
            entry["voted_weight"] += vote_weight(user)
    for vote in state["cast_votes"]:
        if vote["voter_id_string"] in target_of:
            summary[name_of[target_of[vote["voter_id_string"]]]]["cast_votes"] += 1
    warnings = [
        f"area {name!r} has {entry['voted']} voter(s) who voted: per-area results make their "
        f"votes close to individually attributable"
        for name, entry in sorted(summary.items()) if 0 < entry["voted"] < MIN_ANONYMITY_VOTERS
    ]
    plan = {
        "tenant_id": tenant_id,
        "election_event_id": election_event_id,
        "source_area_id": source_area_id,
        "attribute": attribute,
        "blank_policy": blank_policy.value,
        "area_ids": dict(sorted(target_ids.items())),
        "areas_to_create": areas_to_create,
        "contests_to_create": contests_to_create,
        "user_moves": user_moves,
        "vote_moves": {target: sorted(ids) for target, ids in sorted(vote_moves.items())},
        "summary": dict(sorted(summary.items())),
        "counts": {
            "voters_in_scope": len(in_scope),
            "voters_kept_in_source_area": len(blank_users),
            "voters_out_of_scope": out_of_scope,
            "areas": len(target_ids),
            "areas_to_create": len(areas_to_create),
            "contests_to_create": len(contests_to_create),
            "users_to_move": len(user_moves),
            "cast_votes_to_move": sum(len(ids) for ids in vote_moves.values()),
        },
        "warnings": warnings,
        "voter_ids": sorted(in_scope),
        "required_styles": sorted(
            [election_of[contest_id], area_id, contest_id]
            for area_id in set(target_of.values())
            for contest_id in source_contests
        ),
    }
    plan["digest"] = canonical_digest({k: v for k, v in plan.items() if k != "warnings"})
    return plan


def write_private_json(path, value, exclusive):
    """Write and fsync a file only the operator can read."""
    flags = os.O_WRONLY | os.O_CREAT | (os.O_EXCL if exclusive else os.O_TRUNC)
    fd = os.open(path, flags, 0o600)
    with os.fdopen(fd, "w") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.flush()
        os.fsync(handle.fileno())
    directory = os.open(os.path.dirname(os.path.abspath(path)), os.O_RDONLY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def move_user(keycloak, realm, user_id, expected, target):
    """Point one voter at `target` if it is still at `expected`; never overwrite anything else."""
    user = keycloak.get_user(realm, user_id)
    current = single_attribute(user, AREA_ID_ATTRIBUTE)
    if current == target:
        return False
    if current != expected:
        raise RegroupError(f"user {user_id} changed concurrently: area {current}, expected {expected}")
    before = dict(user.get("attributes") or {})
    user["attributes"] = {**before, AREA_ID_ATTRIBUTE: [target]}
    keycloak.put_user(realm, user)
    after = keycloak.get_user(realm, user_id).get("attributes") or {}
    if after != {**before, AREA_ID_ATTRIBUTE: [target]}:
        raise RegroupError(f"user {user_id} attributes differ from the intended update after saving")
    return True


def compare_fingerprints(before, after):
    problems = []
    for table, rows in before.items():
        current = after.get(table, {})
        changed = [row for row, digest in rows.items() if current.get(row) != digest]
        if changed:
            problems.append(f"{len(changed)} pre-existing {table} row(s) changed or disappeared")
    return problems


def actual_summary(state, users, area_ids, source_area_id, source_name):
    """Per-area counts read from where voters and cast votes are now, not from the plan."""
    name_of = {area_id: name for name, area_id in area_ids.items()}
    name_of[source_area_id] = source_name
    summary = defaultdict(lambda: {"voters": 0, "voted": 0, "voted_weight": 0, "cast_votes": 0})
    voted = {v["voter_id_string"] for v in state["cast_votes"] if v["status"] == VALID}
    for user in users:
        name = name_of.get(single_attribute(user, AREA_ID_ATTRIBUTE))
        if name is None:
            continue
        summary[name]["voters"] += 1
        if user["id"] in voted:
            summary[name]["voted"] += 1
            summary[name]["voted_weight"] += vote_weight(user)
    for vote in state["cast_votes"]:
        if vote["area_id"] in name_of:
            summary[name_of[vote["area_id"]]]["cast_votes"] += 1
    return {name: entry for name, entry in sorted(summary.items()) if any(entry.values())}


def current_plan(context, new_area_ids=None):
    state = context.hasura.load(context.tenant_id, context.election_event_id)
    if not state["event"]:
        raise RegroupError(f"election event {context.election_event_id} not found in tenant "
                           f"{context.tenant_id}")
    users = context.keycloak.list_users(event_realm(context.tenant_id, context.election_event_id))
    return state, users, build_plan(
        state, users, context.tenant_id, context.election_event_id, context.source_area_id,
        context.attribute, context.blank_policy, new_area_ids,
    )


def read_json(path):
    with open(path) as handle:
        return json.load(handle)


def require_same_scope(context, recorded):
    for key in ("tenant_id", "election_event_id", "source_area_id", "attribute", "blank_policy"):
        ours = getattr(context, key)
        ours = ours.value if isinstance(ours, BlankPolicy) else ours
        if recorded.get(key) != ours:
            raise RegroupError(f"{key} differs from the reviewed plan")


def command_plan(context, out):
    _, _, plan = current_plan(context)
    write_private_json(context.plan_path, plan, exclusive=False)
    report(out, "PLANNED", plan["counts"], plan["summary"], plan["warnings"])
    return 0


def command_apply(context, out):
    reviewed = read_json(context.plan_path)
    require_same_scope(context, reviewed)
    _, users, plan = current_plan(context, reviewed["area_ids"])
    if plan["digest"] != reviewed["digest"]:
        raise RegroupError("the event changed since the plan was reviewed; plan again")
    realm = event_realm(context.tenant_id, context.election_event_id)
    journal = {
        "plan": plan,
        "fingerprints": context.hasura.fingerprints(context.tenant_id, context.election_event_id),
        "voters": voter_invariants(users, set(plan["voter_ids"])),
        "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    write_private_json(context.journal_path, journal, exclusive=True)

    created, contests, moved = context.hasura.apply(
        context.tenant_id, context.election_event_id, context.source_area_id,
        plan["areas_to_create"], plan["contests_to_create"], plan["vote_moves"],
    )
    conflicts = [f"{len(ids) - moved.get(target, 0)} cast vote(s) for area {target} were not "
                 f"in the source area any more" for target, ids in plan["vote_moves"].items()
                 if moved.get(target, 0) != len(ids)]
    if created != len(plan["areas_to_create"]) or contests != len(plan["contests_to_create"]):
        conflicts.append("area or contest inserts did not match the plan")
    if conflicts:
        raise RegroupError(conflicts + ["database changes are committed; Keycloak was not changed. "
                                        "Run verify, then plan and apply again or revert"])
    for move in plan["user_moves"]:
        move_user(context.keycloak, realm, move["user_id"], move["from"], move["to"])
    return command_verify(context, out, journal)


def command_verify(context, out, journal=None):
    journal = journal or read_json(context.journal_path)
    reviewed = journal["plan"]
    require_same_scope(context, reviewed)
    state, users, plan = current_plan(context, reviewed["area_ids"])
    source_name = next(a["name"] for a in state["areas"] if a["id"] == context.source_area_id)
    actual = actual_summary(state, users, plan["area_ids"], context.source_area_id, source_name)
    problems = []
    if plan["counts"]["areas_to_create"] or plan["counts"]["contests_to_create"]:
        problems.append("some areas or area contests are still missing")
    if plan["user_moves"] or plan["vote_moves"]:
        problems.append(f"{len(plan['user_moves'])} voter(s) and "
                        f"{plan['counts']['cast_votes_to_move']} cast vote(s) are not in their area yet")
    if actual != reviewed["summary"]:
        problems.append("actual per-area counts differ from the reviewed plan")
    changed_voters = [uid for uid, digest in journal["voters"].items()
                      if voter_invariants(users, {uid}).get(uid) != digest]
    if changed_voters:
        problems.append(f"{len(changed_voters)} voter(s) changed in something other than their area "
                        f"(enabled, username, weight, eligibility or another attribute)")
    problems += compare_fingerprints(
        journal["fingerprints"],
        context.hasura.fingerprints(context.tenant_id, context.election_event_id),
    )
    if problems:
        raise RegroupError(problems)
    expected = {tuple(item) for item in plan["required_styles"]}
    regrouped = {area for _, area, _ in expected}
    live = {tuple(item) for item in state["live_styles"]}
    missing = expected - live
    unexpected = {item for item in live if item[1] in regrouped} - expected
    if unexpected:
        raise RegroupError(f"{len(unexpected)} published (election, area, contest) combination(s) "
                           f"of the regrouped areas were not expected; the publication is not "
                           f"the one this regrouping needs")
    if missing:
        report(out, "PUBLICATION_PENDING", plan["counts"], plan["summary"], plan["warnings"],
               [f"{len(missing)} (election, area, contest) combination(s) have no published "
                f"ballot style: generate and publish the ballot publication before any tally"])
        return EXIT_PUBLICATION_PENDING
    report(out, "READY_FOR_TALLY", plan["counts"], plan["summary"], plan["warnings"])
    return 0


def command_revert(context, out):
    """Move every journaled voter and cast vote back, only where nothing else changed them."""
    journal = read_json(context.journal_path)
    plan = journal["plan"]
    require_same_scope(context, plan)
    state = context.hasura.load(context.tenant_id, context.election_event_id)
    if not state["event"]:
        raise RegroupError("election event not found in tenant")
    problems = operational_problems(state)
    if problems:
        raise RegroupError(problems)
    realm = event_realm(context.tenant_id, context.election_event_id)
    conflicts = []
    for move in plan["user_moves"]:
        try:
            move_user(context.keycloak, realm, move["user_id"], move["to"], move["from"])
        except RegroupError as error:
            conflicts += error.problems
    for target, ids in plan["vote_moves"].items():
        context.hasura.apply(context.tenant_id, context.election_event_id, target, [], [],
                             {context.source_area_id: ids})

    users = {u["id"]: u for u in context.keycloak.list_users(realm)}
    votes = {v["id"]: v for v in
             context.hasura.load(context.tenant_id, context.election_event_id)["cast_votes"]}
    for move in plan["user_moves"]:
        user = users.get(move["user_id"])
        if not user or single_attribute(user, AREA_ID_ATTRIBUTE) != move["from"]:
            conflicts.append(f"user {move['user_id']} is not back in area {move['from']}")
    for ids in plan["vote_moves"].values():
        stray = [i for i in ids if votes.get(i, {}).get("area_id") != context.source_area_id]
        if stray:
            conflicts.append(f"{len(stray)} cast vote(s) are not back in the source area")
    if conflicts:
        raise RegroupError(conflicts + ["revert is incomplete; nothing that changed concurrently "
                                        "was overwritten"])
    report(out, "REVERTED", {"users": len(plan["user_moves"]),
                             "cast_votes": plan["counts"]["cast_votes_to_move"]}, {}, [
        "areas created by apply were left in place; republish before any tally"])
    return 0


def report(out, outcome, counts, summary, warnings, notes=()):
    out.write(json.dumps({"outcome": outcome, "counts": counts, "areas": summary,
                          "warnings": list(warnings), "notes": list(notes)}, indent=2) + "\n")


class Context:
    def __init__(self, args, hasura, keycloak):
        self.tenant_id = parse_uuid(args.tenant_id, "--tenant-id")
        self.election_event_id = parse_uuid(args.election_event_id, "--election-event-id")
        self.source_area_id = parse_uuid(args.source_area_id, "--source-area-id")
        self.attribute = args.attribute
        self.blank_policy = BlankPolicy(args.blank_policy)
        self.plan_path = args.plan
        self.journal_path = args.journal
        self.hasura = hasura
        self.keycloak = keycloak


def parse_args(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", choices=["plan", "apply", "verify", "revert"])
    parser.add_argument("--hasura-endpoint", required=True)
    parser.add_argument("--hasura-source", default="backend-db",
                        help="Hasura database source name, used for read-only fingerprints")
    parser.add_argument("--keycloak-url", required=True)
    parser.add_argument("--keycloak-admin-realm", default="master")
    parser.add_argument("--tenant-id", required=True)
    parser.add_argument("--election-event-id", required=True)
    parser.add_argument("--source-area-id", required=True)
    parser.add_argument("--attribute", required=True)
    parser.add_argument("--blank-policy", choices=[p.value for p in BlankPolicy],
                        default=BlankPolicy.FAIL.value)
    parser.add_argument("--plan", required=True, help="plan file written by plan, read by apply")
    parser.add_argument("--journal", help="recovery journal written by apply before any change")
    args = parser.parse_args(argv)
    if args.command != "plan" and not args.journal:
        parser.error(f"{args.command} needs --journal")
    return args


def keycloak_credentials(environ):
    if environ.get("KEYCLOAK_ADMIN_CLIENT_ID") and environ.get("KEYCLOAK_ADMIN_CLIENT_SECRET"):
        return {"grant_type": "client_credentials",
                "client_id": environ["KEYCLOAK_ADMIN_CLIENT_ID"],
                "client_secret": environ["KEYCLOAK_ADMIN_CLIENT_SECRET"]}
    if environ.get("KEYCLOAK_ADMIN_USER") and environ.get("KEYCLOAK_ADMIN_PASSWORD"):
        return {"grant_type": "password", "client_id": "admin-cli",
                "username": environ["KEYCLOAK_ADMIN_USER"],
                "password": environ["KEYCLOAK_ADMIN_PASSWORD"]}
    raise RegroupError("set KEYCLOAK_ADMIN_CLIENT_ID and KEYCLOAK_ADMIN_CLIENT_SECRET, "
                       "or KEYCLOAK_ADMIN_USER and KEYCLOAK_ADMIN_PASSWORD")


COMMANDS = {"plan": command_plan, "apply": command_apply,
            "verify": command_verify, "revert": command_revert}


def main(argv=None, environ=os.environ, out=sys.stdout):
    args = parse_args(argv)
    try:
        secret = environ.get("HASURA_ADMIN_SECRET")
        if not secret:
            raise RegroupError("set HASURA_ADMIN_SECRET")
        context = Context(
            args,
            Hasura(args.hasura_endpoint, secret, args.hasura_source),
            Keycloak(args.keycloak_url, args.keycloak_admin_realm, keycloak_credentials(environ)),
        )
        return COMMANDS[args.command](context, out)
    except RegroupError as error:
        out.write(json.dumps({"outcome": "REFUSED", "problems": error.problems}, indent=2) + "\n")
        return 2
    except (urllib.error.URLError, FileExistsError, FileNotFoundError) as error:
        out.write(json.dumps({"outcome": "ERROR", "problems": [str(error)]}, indent=2) + "\n")
        return 1


if __name__ == "__main__":
    sys.exit(main())
