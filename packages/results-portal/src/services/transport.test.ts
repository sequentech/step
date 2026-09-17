// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, describe, expect, it, jest} from "@jest/globals"
import {parse} from "graphql"
import {graphqlFetch} from "./graphql"
import {resolveSqliteArtifactUrl} from "./artifacts"
import {joinUrl, publicBucketUrl} from "./urls"
import type {ResultsManifest} from "@/types/results"
import type {GlobalSettings} from "@/providers/SettingsContextProvider"

const settings = {
    HASURA_URL: "https://api.invalid/graphql",
    PUBLIC_BUCKET_URL: "https://files.invalid/",
} as GlobalSettings
const manifest = (artifact = {}): ResultsManifest =>
    ({
        schema_version: 1,
        tenant_id: "tenant",
        election_event_id: "event",
        election_ids: ["election"],
        route_scope: "event",
        publication_id: "publication",
        results_event_id: "results",
        version: 1,
        access: "public",
        visibility_scope: "full_event",
        artifacts: {full_sqlite: artifact},
    }) as ResultsManifest
afterEach(() => {
    jest.restoreAllMocks()
})

describe("GraphQL transport", () => {
    it("sends literal variables and adds authorization only when provided", async () => {
        const fetch = jest
            .spyOn(globalThis, "fetch")
            .mockImplementation(
                async () => new Response(JSON.stringify({data: {lookup: "answer"}}))
            )
        const query = parse("query Lookup($id: ID!) { lookup(id: $id) }")
        await expect(
            graphqlFetch(settings.HASURA_URL, query, {id: "row-7"}, "synthetic-token")
        ).resolves.toEqual({lookup: "answer"})
        expect(fetch).toHaveBeenCalledWith(settings.HASURA_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                "Authorization": "Bearer synthetic-token",
            },
            body: JSON.stringify({
                query: "query Lookup($id: ID!) {\n  lookup(id: $id)\n}",
                variables: {id: "row-7"},
            }),
        })
        await graphqlFetch(settings.HASURA_URL, query, {id: "row-8"})
        expect(fetch.mock.calls[1][1]?.headers).toEqual({"Content-Type": "application/json"})
    })
    it.each([
        [new Response("not JSON", {status: 503}), "GraphQL request failed with HTTP 503"],
        [
            new Response(
                JSON.stringify({
                    data: {partial: true},
                    errors: [{message: "first"}, {message: "second"}],
                })
            ),
            "first; second",
        ],
        [new Response(JSON.stringify({data: null})), "GraphQL request returned no data"],
        [new Response("{}"), "GraphQL request returned no data"],
    ])("rejects a failed response without treating it as data", async (response, error) => {
        jest.spyOn(globalThis, "fetch").mockResolvedValue(response as Response)
        await expect(graphqlFetch(settings.HASURA_URL, parse("{ lookup }"), {})).rejects.toThrow(
            error as string
        )
    })
    it("propagates network and JSON decoding failures", async () => {
        const fetch = jest
            .spyOn(globalThis, "fetch")
            .mockRejectedValue(new Error("synthetic offline"))
        await expect(graphqlFetch(settings.HASURA_URL, parse("{ lookup }"), {})).rejects.toThrow(
            "synthetic offline"
        )
        fetch.mockResolvedValue(new Response("invalid JSON"))
        await expect(
            graphqlFetch(settings.HASURA_URL, parse("{ lookup }"), {})
        ).rejects.toMatchObject({name: "SyntaxError"})
    })
})

describe("artifact access", () => {
    it("uses direct/public artifact URLs without requesting authenticated access", async () => {
        const fetch = jest.spyOn(globalThis, "fetch")
        await expect(
            resolveSqliteArtifactUrl(
                settings,
                manifest({url: "https://signed.invalid/result", public_path: "ignored"})
            )
        ).resolves.toBe("https://signed.invalid/result")
        await expect(
            resolveSqliteArtifactUrl(settings, manifest({public_path: "/results/data.sqlite"}))
        ).resolves.toBe("https://files.invalid/results/data.sqlite")
        expect(fetch).not.toHaveBeenCalled()
        await expect(resolveSqliteArtifactUrl(settings, manifest())).rejects.toThrow(
            "requires sign-in"
        )
        expect(fetch).not.toHaveBeenCalled()
    })
    it("requests the selected publication and election with the supplied token", async () => {
        const fetch = jest
            .spyOn(globalThis, "fetch")
            .mockResolvedValue(
                new Response(
                    JSON.stringify({
                        data: {
                            fetchResultsArtifact: {
                                urls: [
                                    "https://signed.invalid/first",
                                    "https://signed.invalid/second",
                                ],
                            },
                        },
                    })
                )
            )
        await expect(
            resolveSqliteArtifactUrl(settings, manifest(), "synthetic-token", "election")
        ).resolves.toBe("https://signed.invalid/first")
        const options = fetch.mock.calls[0][1]!
        expect(options.headers).toEqual({
            "Content-Type": "application/json",
            "Authorization": "Bearer synthetic-token",
        })
        expect(JSON.parse(options.body as string).variables).toEqual({
            electionEventId: "event",
            electionId: "election",
            publicationId: "publication",
        })
    })
    it.each([null, {urls: []}])(
        "rejects inaccessible authenticated artifacts",
        async (artifact) => {
            jest.spyOn(globalThis, "fetch").mockResolvedValue(
                new Response(JSON.stringify({data: {fetchResultsArtifact: artifact}}))
            )
            await expect(
                resolveSqliteArtifactUrl(settings, manifest(), "synthetic-token")
            ).rejects.toThrow("No accessible results artifact")
        }
    )
})

it("joins bucket paths without rewriting absolute HTTP URLs", () => {
    expect(joinUrl("https://files.invalid///", "///a/b.sqlite")).toBe(
        "https://files.invalid/a/b.sqlite"
    )
    expect(joinUrl("https://files.invalid/", "HTTPS://signed.invalid/a?token=x")).toBe(
        "HTTPS://signed.invalid/a?token=x"
    )
    expect(publicBucketUrl("https://files.invalid/", "a")).toBe("https://files.invalid/a")
    expect(publicBucketUrl("https://files.invalid/")).toBeUndefined()
    expect(publicBucketUrl("https://files.invalid/", "")).toBeUndefined()
})
