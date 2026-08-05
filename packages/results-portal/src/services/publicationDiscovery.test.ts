// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, it} from "@jest/globals"
import {parseResultsManifest, ResultsPublicationIndex} from "@/types/results"
import {findIndexPublication} from "./publicationDiscovery"

const eventId = "event-1"
const electionId = "election-1"

describe("findIndexPublication", () => {
    it("does not expose an event publication on an unrelated election route", () => {
        const index: ResultsPublicationIndex = {
            schema_version: 1,
            publications: [
                {
                    publication_id: "event-publication",
                    route_scope: "event",
                    route: `/${eventId}`,
                    election_ids: [electionId],
                    access: "public",
                },
            ],
        }

        expect(findIndexPublication(index, eventId, "unpublished-election")).toBeNull()
    })

    it("prefers the exact election publication over the event fallback", () => {
        const index: ResultsPublicationIndex = {
            schema_version: 1,
            publications: [
                {
                    publication_id: "event-publication",
                    route_scope: "event",
                    route: `/${eventId}`,
                    election_ids: [electionId],
                    access: "public",
                },
                {
                    publication_id: "election-publication",
                    route_scope: "election",
                    route_election_id: electionId,
                    election_ids: [electionId],
                    access: "public",
                },
            ],
        }

        expect(findIndexPublication(index, eventId, electionId)?.publication_id).toBe(
            "election-publication"
        )
    })
})

describe("parseResultsManifest", () => {
    it("accepts legacy serde manifests with null optional artifacts", () => {
        expect(
            parseResultsManifest({
                schema_version: 1,
                tenant_id: "tenant-1",
                election_event_id: eventId,
                election_ids: [electionId],
                route_scope: "event",
                route_election_id: null,
                publication_id: "publication-1",
                results_event_id: "results-event-1",
                version: 1,
                access: "public",
                visibility_scope: "full_event",
                contests: [],
                artifacts: {
                    full_sqlite: {
                        public_path: "results/full-v1.sqlite",
                        document_id: "document-1",
                    },
                    areas: null,
                },
            }).publication_id
        ).toBe("publication-1")
    })
})
