#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Render only the demo action files to a fresh directory; never apply metadata."""
import argparse
from pathlib import Path

ACTIONS = {
    "operationalInitializeStatus": ("query", "/status", "request_id: uuid!", "OperationalInitializeStatusOutput"),
    "operationalInitializeSetupStatus": ("query", "/setup/status", "request_id: uuid!", "OperationalInitializeSetupView"),
    "operationalInitializeSetup": ("mutation", "/setup", "request_id: uuid!", "OperationalInitializeSetupView"),
    "operationalInitializeStart": ("mutation", "/start", "request_id: uuid!, page: OperationalInitializeReturnPageInput!, payload_sha256: String!", "OperationalInitializeStartOutput"),
    "operationalInitializeFinish": ("mutation", "/finish", "request_id: uuid!, attempt: uuid!, state: String!, code: String!", "OperationalInitializeGoldReturn"),
    "operationalInitializeCancel": ("mutation", "/cancel", "request_id: uuid!, attempt: uuid!", "OperationalInitializeOutcome"),
    "operationalInitializeReconcile": ("mutation", "/reconcile", "request_id: uuid!", "OperationalInitializeOutcome"),
}
PAGE = {"path": "String!", "panel": "String!", "election_id": "uuid"}
OBJECTS = {
    "OperationalInitializeScope": {key: "uuid!" for key in ["tenant", "event", "request", "official"]},
    "OperationalInitializeStatusOutput": {"scope": "OperationalInitializeScope!", "phase": "String!", "cancellable_attempt": "uuid", "device_profile": "String!"},
    "OperationalInitializeSetupView": {"scope": "OperationalInitializeScope!", "phase": "String!"},
    "OperationalInitializeStartOutput": {"attempt": "uuid!", "redirect": "String!"},
    "OperationalInitializeOutcome": {"phase": "String!"},
    "OperationalInitializeReturnPage": PAGE,
    "OperationalInitializeGoldReturn": {
        **{key: "uuid!" for key in ["tenant", "event", "request", "official", "attempt"]},
        "phase": "String!", "page": "OperationalInitializeReturnPage!",
    },
}
INPUT = "OperationalInitializeReturnPageInput"


def yaml_type(name, fields):
    return f"        - name: {name}\n          fields:\n" + "".join(
        f"              - name: {field}\n                type: {kind}\n" for field, kind in fields.items()
    )


def graphql_type(kind, name, fields):
    body = "".join(f"    {field}: {spec}\n" for field, spec in fields.items())
    return f"{kind} {name} {{\n{body}}}\n"


def insert(source, anchor, extra):
    if source.count(anchor) != 1:
        raise ValueError(f"expected a single original Hasura marker: {anchor!r}")
    return source.replace(anchor, extra + anchor, 1)


def render(graphql, yaml):
    if any(name in graphql or name in yaml for name in ACTIONS):
        raise ValueError("input already contains operational demo actions")
    definitions = ""
    for kind in ("query", "mutation"):
        definitions += graphql_type("type", kind.title(), {
            f"{name}({args})": f"{output}!"
            for name, (action_kind, _path, args, output) in ACTIONS.items() if kind == action_kind
        })
    definitions += graphql_type("input", INPUT, PAGE)
    definitions += "".join(graphql_type("type", name, fields) for name, fields in OBJECTS.items())

    actions = ""
    for name, (kind, path, _args, _output) in ACTIONS.items():
        setting = "type: query" if kind == "query" else "kind: synchronous"
        actions += f"""    - name: {name}
      definition:
          {setting}
          handler: http://{{{{OPERATIONAL_DEMO_HARVEST_DOMAIN}}}}/operational-signing/initialize{path}
          forward_client_headers: true
          request_transform:
              body:
                  action: transform
                  template: "{{{{$body.input}}}}"
              template_engine: Kriti
              version: 2
      permissions:
          - role: sign-initialize-voting
"""
    # The two brace pairs here are literal Kriti interpolation, not a Python
    # interpolation. No x-hasura-session headers are interpreted by Harvest.
    yaml = insert(yaml, "\ncustom_types:\n", "\n" + actions.rstrip() )
    # Append each new custom type to its existing, standard metadata array.
    yaml = insert(yaml, "\n    objects:\n", "\n" + yaml_type(INPUT, PAGE).rstrip())
    yaml = insert(yaml, "\n    scalars:", "\n" + "".join(yaml_type(n, f) for n, f in OBJECTS.items()).rstrip())
    return graphql.rstrip() + "\n\n" + definitions, yaml


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("metadata", type=Path, help="original directory containing actions.graphql and actions.yaml")
    parser.add_argument("output", type=Path, help="new directory; must not exist")
    args = parser.parse_args()
    g, y = render(*((args.metadata / name).read_text() for name in ("actions.graphql", "actions.yaml")))
    args.output.mkdir(mode=0o700)
    (args.output / "actions.graphql").write_text(g)
    (args.output / "actions.yaml").write_text(y)


if __name__ == "__main__":
    main()
