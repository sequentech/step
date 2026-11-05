// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic ballot box seals (VOTE-FREEZE) of the issue's drafts: a Post
// closed by two signers and sealed in a zone with no grace period, one whose
// second seal is still being posted, and a staff association that closes
// with a 15-minute grace period and an administrator's Stop Voting. Every
// value is invented; the hashes are not real seals.
import {EBallotBoxSealPolicy, type IElectionEventPresentation} from "@sequentech/ui-core"
import {STORY_IDS, eventPresentation, storyId} from "@/__stories__/fixtures"
import {
    EBallotBoxClosedByKind,
    EBallotBoxSealStatus,
    type IBallotBoxClosedBy,
    type IBallotBoxSeal,
} from "@/types/ballotBoxSeal"

export const MANILA = "Asia/Manila"
export const MADRID = "Europe/Madrid"

/** An event that seals its ballot boxes at close, with its times in `zone`. */
export const sealingEventPresentation = (
    zone: string,
    policy: EBallotBoxSealPolicy = EBallotBoxSealPolicy.SEAL_AT_CLOSE
): IElectionEventPresentation => ({
    ...eventPresentation,
    timezones: {configured: [zone], primary: zone},
    ballot_box_seal_policy: policy,
})

export interface IBallotBoxArea {
    id: string
    name: string
}

export const AREAS = {
    first: {id: STORY_IDS.area, name: "Spain"},
    second: {id: STORY_IDS.secondArea, name: "Andorra"},
    madridOffice: {id: STORY_IDS.area, name: "Madrid office"},
    canaryOffice: {id: STORY_IDS.secondArea, name: "Canary Islands office"},
} as const

export const SIGNED_CLOSE: IBallotBoxClosedBy = {
    kind: EBallotBoxClosedByKind.SIGNED,
    signers: [
        {name: "Maria L. Santos", certificate_sha256: "a1".repeat(32)},
        {name: "Jose R. Dela Cruz", certificate_sha256: "b2".repeat(32)},
    ],
    signing_code: "5D90-A3F7",
}

export const ADMIN_CLOSE: IBallotBoxClosedBy = {
    kind: EBallotBoxClosedByKind.USER,
    username: "rrhh.admin",
}

export const SCHEDULED_CLOSE: IBallotBoxClosedBy = {kind: EBallotBoxClosedByKind.SCHEDULED}

/** 7:01 PM PhST, with no grace period. */
export const POST_CLOSE = "2028-05-08T11:01:00Z"
/** The private documents of the restricted seal records, one per area. */
export const RESTRICTED_RECORD_DOCUMENTS = [storyId(9, 1), storyId(9, 2)]
/** 6:00 PM CET and the end of its 15-minute grace period, 6:15 PM CET. */
export const ASSOCIATION_CLOSE = "2028-03-13T17:00:00Z"
export const ASSOCIATION_DEADLINE = "2028-03-13T17:15:00Z"

export function sealRow(
    area: IBallotBoxArea,
    status: EBallotBoxSealStatus,
    overrides: Partial<IBallotBoxSeal> = {}
): IBallotBoxSeal {
    const sealed =
        status === EBallotBoxSealStatus.SEALED || status === EBallotBoxSealStatus.PUBLISHED
    const closedAt = overrides.closed_at ?? POST_CLOSE
    const deadline = overrides.grace_deadline ?? closedAt
    return {
        id: storyId(0, area.id === STORY_IDS.area ? 1 : 2),
        election_id: STORY_IDS.election,
        area_id: area.id,
        area: {id: area.id, name: area.name},
        status,
        closed_at: closedAt,
        grace_deadline: deadline,
        closed_by: SIGNED_CLOSE,
        sealed_at: sealed ? deadline : null,
        ballots_in_box: sealed ? 1342 : null,
        ballots_counted: sealed ? 1340 : null,
        seal_hash: sealed ? `ef187f0b${"3c".repeat(56)}22a65e5b` : null,
        failure_reason: null,
        public_path:
            status === EBallotBoxSealStatus.PUBLISHED
                ? `ballot-box-seals/${STORY_IDS.election}/${area.id}.json`
                : null,
        published_at: status === EBallotBoxSealStatus.PUBLISHED ? deadline : null,
        ...overrides,
    }
}

/** The scenarios of the card, as the drafts show them. */
export enum EBallotBoxesScenario {
    OPEN = "open",
    NOT_STARTED = "notStarted",
    HOLDING = "holding",
    SEALED = "sealed",
    PUBLISHING = "publishing",
    GRACE = "grace",
    GRACE_SEALED = "graceSealed",
    DUE = "due",
    OVERDUE_CHANNEL = "overdueChannel",
    OVERDUE_NOT_ENABLED = "overdueNotEnabled",
    OVERDUE_BALLOTS = "overdueBallots",
    OVERDUE_DATAFIX = "overdueDatafix",
    OVERDUE_ERROR = "overdueError",
    OVERDUE_STALE = "overdueStale",
    FAILED = "failed",
    /** Sealed, with restricted (private) seal records: no public path. */
    RESTRICTED_RECORD = "restrictedRecord",
}

/** An election's status and channels, as the card reads them before the close. */
export interface IFixtureElection {
    status: Record<string, unknown>
    voting_channels: Record<string, boolean>
}

const ONLINE_ONLY = {online: true, kiosk: false, early_voting: false, telephone: false}
const ONLINE_AND_KIOSK = {...ONLINE_ONLY, kiosk: true}

export interface IBallotBoxesFixture {
    zone: string
    seals: IBallotBoxSeal[]
    /** The areas of the election's published ballot styles. */
    areas: IBallotBoxArea[]
    election: IFixtureElection
    /** The time the card shows the statuses at. */
    now: string
}

const association = (
    status: EBallotBoxSealStatus,
    overrides: Partial<IBallotBoxSeal> = {}
): IBallotBoxSeal[] =>
    [AREAS.madridOffice, AREAS.canaryOffice].map((area, index) =>
        sealRow(area, status, {
            closed_at: ASSOCIATION_CLOSE,
            grace_deadline: ASSOCIATION_DEADLINE,
            closed_by: ADMIN_CLOSE,
            ...(status === EBallotBoxSealStatus.PUBLISHED
                ? {
                      ballots_in_box: index ? 37 : 212,
                      ballots_counted: index ? 37 : 208,
                      seal_hash: index
                          ? `7f9533b3${"5a".repeat(56)}fd796f60`
                          : `109a3264${"6b".repeat(56)}efd2b5d8`,
                  }
                : {}),
            ...overrides,
        })
    )

const POST_AREAS = [AREAS.first, AREAS.second]
const OFFICES = [AREAS.madridOffice, AREAS.canaryOffice]
const closedOnline: IFixtureElection = {
    status: {voting_status: "CLOSED"},
    voting_channels: ONLINE_ONLY,
}
/** Pending past its deadline, with the sealer's last attempt and why it waits (D5). */
const overdue = (waiting_reason: string | null, last_attempt_at: string | null) =>
    POST_AREAS.map((area) =>
        sealRow(area, EBallotBoxSealStatus.PENDING, {waiting_reason, last_attempt_at})
    )

export const BALLOT_BOXES_FIXTURES: Record<EBallotBoxesScenario, IBallotBoxesFixture> = {
    [EBallotBoxesScenario.OPEN]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: {status: {voting_status: "OPEN"}, voting_channels: ONLINE_ONLY},
        seals: [],
        now: "2028-05-08T09:00:00Z",
    },
    [EBallotBoxesScenario.NOT_STARTED]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: {status: {voting_status: "NOT_STARTED"}, voting_channels: ONLINE_ONLY},
        seals: [],
        now: "2028-05-01T09:00:00Z",
    },
    [EBallotBoxesScenario.HOLDING]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: {
            status: {voting_status: "CLOSED", kiosk_voting_status: "NOT_STARTED"},
            voting_channels: ONLINE_AND_KIOSK,
        },
        seals: [],
        now: "2028-05-08T11:10:00Z",
    },
    [EBallotBoxesScenario.SEALED]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: [
            sealRow(AREAS.first, EBallotBoxSealStatus.PUBLISHED),
            sealRow(AREAS.second, EBallotBoxSealStatus.PUBLISHED, {
                ballots_in_box: 16,
                ballots_counted: 16,
                seal_hash: `ffdbf015${"4d".repeat(56)}19d9a202`,
            }),
        ],
        now: "2028-05-08T11:10:00Z",
    },
    [EBallotBoxesScenario.PUBLISHING]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: [
            sealRow(AREAS.first, EBallotBoxSealStatus.PUBLISHED),
            sealRow(AREAS.second, EBallotBoxSealStatus.SEALED, {
                ballots_in_box: 38,
                ballots_counted: 38,
                seal_hash: `14639763${"7e".repeat(56)}dbb7b3e5`,
            }),
        ],
        now: "2028-05-08T11:10:00Z",
    },
    [EBallotBoxesScenario.GRACE]: {
        zone: MADRID,
        areas: OFFICES,
        election: closedOnline,
        seals: association(EBallotBoxSealStatus.PENDING),
        now: "2028-03-13T17:05:00Z",
    },
    [EBallotBoxesScenario.GRACE_SEALED]: {
        zone: MADRID,
        areas: OFFICES,
        election: closedOnline,
        seals: association(EBallotBoxSealStatus.PUBLISHED),
        now: "2028-03-13T17:20:00Z",
    },
    [EBallotBoxesScenario.DUE]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: overdue("deadline", "2028-05-08T11:01:20Z"),
        now: "2028-05-08T11:01:40Z",
    },
    [EBallotBoxesScenario.OVERDUE_CHANNEL]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: overdue("channel_open:KIOSK", "2028-05-08T11:05:00Z"),
        now: "2028-05-08T11:05:30Z",
    },
    [EBallotBoxesScenario.OVERDUE_NOT_ENABLED]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: overdue("channel_not_enabled:KIOSK", "2028-05-08T11:05:00Z"),
        now: "2028-05-08T11:05:30Z",
    },
    [EBallotBoxesScenario.OVERDUE_BALLOTS]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: overdue("channel_has_ballots:TELEPHONE", "2028-05-08T11:05:00Z"),
        now: "2028-05-08T11:05:30Z",
    },
    [EBallotBoxesScenario.OVERDUE_DATAFIX]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: overdue("datafix_votes:3", "2028-05-08T11:05:00Z"),
        now: "2028-05-08T11:05:30Z",
    },
    [EBallotBoxesScenario.OVERDUE_ERROR]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: overdue("error:keystore", "2028-05-08T11:05:00Z"),
        now: "2028-05-08T11:05:30Z",
    },
    [EBallotBoxesScenario.OVERDUE_STALE]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: overdue(null, null),
        now: "2028-05-08T11:20:00Z",
    },
    [EBallotBoxesScenario.FAILED]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: [
            sealRow(AREAS.first, EBallotBoxSealStatus.PUBLISHED),
            sealRow(AREAS.second, EBallotBoxSealStatus.FAILED, {
                failure_reason:
                    "a ballot does not match its Ballot ID (stored 3a91…, content hashes to 77c0…)",
            }),
        ],
        now: "2028-05-08T11:10:00Z",
    },
    [EBallotBoxesScenario.RESTRICTED_RECORD]: {
        zone: MANILA,
        areas: POST_AREAS,
        election: closedOnline,
        seals: POST_AREAS.map((area, index) =>
            sealRow(area, EBallotBoxSealStatus.PUBLISHED, {
                public_path: null,
                public_document_id: RESTRICTED_RECORD_DOCUMENTS[index],
            })
        ),
        now: "2028-05-08T11:10:00Z",
    },
}
