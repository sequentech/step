// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {Policy} from "@vitrine/verify"
import {explorerLink, explorersOf} from "./explorers"

const policyWith = (fields: Record<string, unknown>) =>
    ({
        operator: "",
        explorers: [],
        monitors: new Map(),
        hubs: new Map(),
        ...fields,
    }) as unknown as Policy

describe("the explorers a policy names", () => {
    it("lists independent monitors' and hubs' first, then others', and the operator's last", () => {
        const policy = policyWith({
            operator: "Election authority",
            explorers: ["https://election.example/"],
            hubs: new Map([
                [
                    "hub-a",
                    {
                        name: "hub-a",
                        operator: "Election authority",
                        explorer: "https://hub-a.example/",
                    },
                ],
                ["hub-b", {name: "hub-b", operator: "Hub B", explorer: "https://hub-b.example/"}],
                ["hub-c", {name: "hub-c", operator: "Hub C"}],
            ]),
            monitors: new Map([
                ["obs", {name: "obs", party: "Observer", explorer: "https://obs.example/"}],
            ]),
        })
        expect(explorersOf(policy)).toEqual([
            {
                url: "https://obs.example/",
                kind: "monitor",
                name: "obs",
                party: "Observer",
                independent: true,
            },
            {
                url: "https://hub-b.example/",
                kind: "hub",
                name: "hub-b",
                party: "Hub B",
                independent: true,
            },
            {
                url: "https://hub-a.example/",
                kind: "hub",
                name: "hub-a",
                party: "Election authority",
                independent: false,
            },
            {
                url: "https://election.example/",
                kind: "operator",
                name: "Election authority",
                party: "Election authority",
                independent: false,
            },
        ])
    })

    it("lists none for a policy without explorer lines, or none at all", () => {
        expect(explorersOf(policyWith({}))).toEqual([])
        expect(explorersOf(undefined)).toEqual([])
    })
})

describe("the link to an explorer", () => {
    const target = {policyId: "sha256:abc", log: "ballots"}

    it("opens a ballot by its Tracking ID", () => {
        expect(explorerLink("https://x.example/", {...target, find: "FTBE-MHRX"})).toBe(
            "https://x.example/#policy=sha256%3Aabc&log=ballots&find=FTBE-MHRX"
        )
    })

    it("opens an entry by its index, and replaces a fragment the address already has", () => {
        expect(explorerLink("https://x.example/#old", {...target, entry: 0})).toBe(
            "https://x.example/#policy=sha256%3Aabc&log=ballots&entry=0"
        )
    })
})
