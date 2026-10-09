# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""The client's lifecycle preset (``templates/<client>/lifecycle.json``): the
event's timezones (configured list, primary, times in logs), its lifecycle
policies (initialization scope, what an unsigned scheduled close does), its
ballot box seal policy and each Post's timezone.

The event gets the preset's ``timezones``, ``lifecycle_policies`` and
``ballot_box_seal_policy`` unless its presentation already has them; the Parameters sheet is applied after
this, so it can still change either. An election gets its Post's zone when
the Posts sheet gave it none. The Post of an election is the part of its
alias before `` - `` (``DUBAI PCG - General Election`` and
``DUBAI PCG - Test Voting`` are both ``DUBAI PCG``).

Kept apart from run.py, which runs a delivery when it is imported, so the
pieces can be tested on their own.
"""

import json
import logging


def load(path):
    with open(path, "r") as file:
        return json.load(file)


def post_of(alias):
    """The Post an election alias names, in upper case, or None."""
    if not isinstance(alias, str) or not alias.strip():
        return None
    return alias.split(" - ")[0].strip().upper()


def post_time_zones(preset):
    """{Post: zone} from the preset."""
    return {post_of(entry["post"]): entry["timezone"] for entry in preset.get("posts", [])}


def apply_event(election_event, preset):
    """The event's presentation with the preset's timezones and policies,
    where it has none of its own."""
    presentation = election_event.setdefault("presentation", {})
    for key in ("timezones", "lifecycle_policies", "ballot_box_seal_policy"):
        if presentation.get(key) is None and key in preset:
            presentation[key] = json.loads(json.dumps(preset[key]))
    return election_event


def apply_posts(elections, preset):
    """Each election without a timezone gets its Post's zone from the
    preset. An election whose Post isn't in the preset keeps none (it uses
    the event's primary zone), with a warning."""
    zones = post_time_zones(preset)
    for election in elections:
        presentation = election.setdefault("presentation", {})
        if presentation.get("timezone"):
            continue
        post = post_of(election.get("alias"))
        zone = zones.get(post)
        if zone:
            presentation["timezone"] = zone
        else:
            logging.warning(
                "Post %r of election %r isn't in the lifecycle preset: it uses the primary timezone",
                post,
                election.get("alias"),
            )
    return elections
