// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {electionFixture, IDS} from "./index"

export const DESIGN_LANGUAGES = ["en", "tl"] as const

/** The colours the published stylesheet sets, as a browser reports them. */
export const DESIGN_COLORS = {
    frame: "rgb(0, 56, 168)",
    contestTitle: "rgb(206, 17, 38)",
} as const

export const DESIGN_CSS = `
.app-root { border-top: 6px solid ${DESIGN_COLORS.frame}; }
.contest-title { color: ${DESIGN_COLORS.contestTitle}; }
`

export const DESIGN_NAMES = {
    event: {en: "National Elections", tl: "Pambansang Halalan"},
    election: {en: "National Ballot", tl: "Pambansang Balota"},
    senator: {en: "Senator", tl: "Senador"},
    partyList: {en: "Party List", tl: "Party-List"},
} as const

const SENATOR_SEATS = 12
const SENATOR_COLUMNS = 3
const PARTY_LIST_COLUMNS = 2

/** Listed out of alphabetical order, which is the order this contest is published with. */
const PARTIES = ["Mabuhay", "Bayanihan", "Kalinga", "Dangal", "Agila", "Sikat"]

const id = (group: number, index: number) =>
    `${group}0000000-0000-4000-8000-${String(index).padStart(12, "0")}`

export const pictureKey = (name: string) =>
    `tenant-${IDS.tenant}/document-${name.toLowerCase().replace(/\W+/g, "-")}/picture.png`

export interface BallotDesignOptions {
    /** An absolute URL, as the event's presentation stores it. */
    logoUrl?: string
    contestsOrder?: "custom" | "alphabetical" | "random"
    /** Single-seat contests added after the two national ones. */
    localContests?: number
}

/**
 * A published ballot shaped like a national one: a twelve-seat contest and a
 * one-seat contest, in two languages, with columns, pictures, a logo and a
 * stylesheet. What a spec expects of the voter's screens is read from `design`.
 */
export function ballotDesignFixture({
    logoUrl,
    contestsOrder = "custom",
    localContests = 0,
}: BallotDesignOptions = {}) {
    const base = electionFixture()
    const scope = {tenant_id: IDS.tenant, election_event_id: IDS.event, election_id: IDS.election}
    const picture = (name: string) => ({
        urls: [{url: pictureKey(name), kind: "image", title: name, is_image: true}],
    })

    // Published last to first, so the order on screen can only be the design's.
    const senators = Array.from({length: SENATOR_SEATS + 3}, (_, index) => {
        const name = `Senatorial Candidate ${String(index + 1).padStart(2, "0")}`
        return {
            ...scope,
            id: id(71, index + 1),
            contest_id: id(61, 1),
            name,
            presentation: {sort_order: SENATOR_SEATS + 3 - index, ...picture(name)},
        }
    })
    const parties = PARTIES.map((name, index) => ({
        ...scope,
        id: id(72, index + 1),
        contest_id: id(61, 2),
        name,
        presentation: {sort_order: index, ...picture(name)},
    }))
    const contest = (
        index: number,
        names: {en: string; tl: string},
        seats: number,
        presentation: {candidates_order: string; columns?: number},
        candidates: Array<Record<string, unknown>>
    ) => ({
        ...base.ballot.contests[0],
        id: id(61, index),
        name: names.en,
        name_i18n: names,
        description: "",
        max_votes: seats,
        min_votes: 0,
        winning_candidates_num: seats,
        presentation: {
            invalid_vote_policy: "not-allowed",
            under_vote_policy: "allowed",
            sort_order: index,
            ...presentation,
        },
        candidates,
    })
    const contests = [
        contest(
            1,
            DESIGN_NAMES.senator,
            SENATOR_SEATS,
            {candidates_order: "custom", columns: SENATOR_COLUMNS},
            senators
        ),
        contest(
            2,
            DESIGN_NAMES.partyList,
            1,
            {candidates_order: "alphabetical", columns: PARTY_LIST_COLUMNS},
            parties
        ),
        ...Array.from({length: localContests}, (_, index) => {
            const name = `Local Office ${index + 1}`
            return contest(index + 3, {en: name, tl: name}, 1, {candidates_order: "custom"}, [
                {
                    ...scope,
                    id: id(73, index + 1),
                    contest_id: id(61, index + 3),
                    name: `${name} Candidate`,
                    presentation: {sort_order: 0},
                },
            ])
        }),
    ]

    const eventPresentation = {
        ...base.event.presentation,
        i18n: {en: {name: DESIGN_NAMES.event.en}, tl: {name: DESIGN_NAMES.event.tl}},
        language_conf: {default_language_code: "en", enabled_language_codes: [...DESIGN_LANGUAGES]},
        logo_url: logoUrl ?? null,
        css: DESIGN_CSS,
    }
    const electionPresentation = {
        ...base.election.presentation,
        i18n: {
            en: {name: DESIGN_NAMES.election.en, description: ""},
            tl: {name: DESIGN_NAMES.election.tl, description: ""},
        },
        contests_order: contestsOrder,
    }
    const ballot = {
        ...base.ballot,
        description: DESIGN_NAMES.election.en,
        election_event_presentation: eventPresentation,
        election_presentation: electionPresentation,
        contests,
    }
    const byOrder = [...senators].sort(
        (a, b) => a.presentation.sort_order - b.presentation.sort_order
    )

    return {
        ...base,
        ballot,
        event: {...base.event, description: DESIGN_NAMES.event.en, presentation: eventPresentation},
        election: {
            ...base.election,
            description: DESIGN_NAMES.election.en,
            presentation: electionPresentation,
        },
        style: {...base.style, ballot_eml: JSON.stringify(ballot)},
        /** What the voter's screens have to show for this publication. */
        design: {
            contests: contests.map((each) => each.name),
            senators: byOrder.map((each) => each.name),
            parties: [...PARTIES].sort(),
            pictures: [...senators, ...parties].map((each) => pictureKey(each.name)),
            columns: {senator: SENATOR_COLUMNS, partyList: PARTY_LIST_COLUMNS},
        },
    }
}
