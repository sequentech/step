// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, type PropsWithChildren} from "react"
import {Outlet} from "react-router"
import {fn} from "storybook/test"
import type {FetchResult, Operation} from "@apollo/client"
import type {DataProvider, RaRecord} from "react-admin"
import {graphqlBoundary, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {ResourceScreen} from "@/__stories__/resourceScreen"
import {
    FIXED_TIME,
    STORY_IDS,
    candidateRecords,
    contestRecord,
    electionRecord,
    eventRecord,
    storyId,
    tenantRecord,
    type StoryRecord,
} from "@/__stories__/fixtures"
import type {Sequent_Backend_Candidate, Sequent_Backend_Document} from "@/gql/graphql"
import {ElectionEventTallyContext} from "@/providers/ElectionEventTallyProvider"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

/** What the candidate screens' services do. */
export interface CandidateServices {
    /** Reading the candidates. */
    reads: ReadState
    /** Whether the tenant has candidates. */
    empty: boolean
    /** A save rejects with this message. */
    writeError?: string
    /** The candidate's picture is stored as a document. */
    withImage: boolean
}

export const RESOURCE = "sequent_backend_candidate"

export const IMAGE_ID = storyId(5, 7)
export const imageDocument: StoryRecord<Sequent_Backend_Document> = {
    id: IMAGE_ID,
    tenant_id: TENANT_ID,
    election_event_id: STORY_IDS.event,
    name: "alice.png",
    media_type: "image/png",
    size: 2048,
    is_public: true,
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
    annotations: {},
    labels: {},
}

/** The shared candidates with the name column Hasura also returns. */
export const candidates = (): StoryRecord<Sequent_Backend_Candidate>[] =>
    candidateRecords().map((candidate, index) => ({
        ...candidate,
        name: ["Alice Example", "Bob Example"][index],
    }))

/** Alice, whose picture is stored when `withImage` is set. */
export function aliceRecord(withImage = false): StoryRecord<Sequent_Backend_Candidate> {
    const [alice] = candidates()
    return withImage
        ? {
              ...alice,
              image_document_id: IMAGE_ID,
              presentation: {
                  ...alice.presentation,
                  urls: [
                      {
                          url: `tenant-${TENANT_ID}/document-${IMAGE_ID}/alice.png`,
                          is_image: true,
                      },
                  ],
              },
          }
        : alice
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let provider: DataProvider

/** NewResourceContext's record of the last created resource. */
export const lastCreated = fn()
/** The tally store's candidate, contest and event of the open candidate. */
export const tallyFlags = {
    candidate: fn(),
    contest: fn(),
    event: fn(),
}

export async function setUpCandidates(
    {reads, empty, writeError, withImage}: CandidateServices,
    operations: Record<string, (operation: Operation) => FetchResult> = {}
) {
    lastCreated.mockClear()
    Object.values(tallyFlags).forEach((flag) => flag.mockClear())
    const [, bob] = candidates()
    data = resourceBoundary(
        {
            [RESOURCE]: empty ? [] : [aliceRecord(withImage), bob],
            sequent_backend_contest: [contestRecord()],
            sequent_backend_election: [electionRecord()],
            sequent_backend_election_event: [eventRecord()],
            sequent_backend_tenant: [tenantRecord],
            sequent_backend_document: withImage ? [imageDocument] : [],
        },
        {reads: {[RESOURCE]: reads}, writeError}
    )
    // Until its contest has loaded, CandidateDataForm reads the election whose
    // ID is the tenant's, which Hasura does not find.
    provider = new Proxy(data.provider, {
        get(target, key, receiver) {
            if (key !== "getOne") return Reflect.get(target, key, receiver)
            return (resource: string, params: {id: unknown}) =>
                resource === "sequent_backend_election" && params.id === TENANT_ID
                    ? Promise.reject(new Error("Not found"))
                    : target.getOne(resource, params as never)
        },
    })
    graphql = graphqlBoundary(
        {
            election_events_tree: () => ({data: {sequent_backend_election_event: []}}),
            ...operations,
        },
        {schema: true}
    )
    await graphql.ready
}

/** The candidate screens with the tree menu, tally store and new-resource contexts. */
export function CandidateScreen({children}: PropsWithChildren) {
    const {permissions, tenant} = useStoryGlobals()
    const tally = useContext(ElectionEventTallyContext)
    return (
        <ResourceScreen
            resource={RESOURCE}
            label="Candidates"
            boundary={graphql}
            dataProvider={provider}
            role={permissions}
            tenant={tenant}
            // Pictures load from the Storybook server, which answers that they are
            // missing, instead of from the public bucket.
            settings={{PUBLIC_BUCKET_URL: `${globalThis.location.origin}/story-bucket/`}}
        >
            <NewResourceContext.Provider
                value={{lastCreatedResource: null, setLastCreatedResource: lastCreated}}
            >
                <ElectionEventTallyContext.Provider
                    value={{
                        ...tally,
                        setCandidateIdFlag: tallyFlags.candidate,
                        setContestIdFlag: tallyFlags.contest,
                        setElectionEventIdFlag: tallyFlags.event,
                    }}
                >
                    {children}
                </ElectionEventTallyContext.Provider>
            </NewResourceContext.Provider>
        </ResourceScreen>
    )
}

/** The screen as the route layout, so that notifications outlive a redirect. */
export function CandidateLayout() {
    return (
        <CandidateScreen>
            <Outlet />
        </CandidateScreen>
    )
}

export const graphqlCalls = () => graphql.calls
export const dataWrites = () => data.writes
export const records = (resource: string): RaRecord[] => data.records[resource]
/** Calls of one data provider method for one resource. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
