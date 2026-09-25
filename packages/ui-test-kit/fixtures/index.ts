// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

export const FIXED_TIME = "2026-01-15T12:00:00.000Z"
export const IDS = {
    tenant: "10000000-0000-4000-8000-000000000001",
    event: "20000000-0000-4000-8000-000000000001",
    election: "30000000-0000-4000-8000-000000000001",
    area: "40000000-0000-4000-8000-000000000001",
    style: "50000000-0000-4000-8000-000000000001",
    contest: "60000000-0000-4000-8000-000000000001",
    alice: "70000000-0000-4000-8000-000000000001",
    bob: "70000000-0000-4000-8000-000000000002",
    voter: "80000000-0000-4000-8000-000000000001",
} as const

/** Synthetic wire documents: expectations in specs use the literal voter choices, not these builders. */
export function electionFixture({demo = false, gold = false, finishUrl = ""} = {}) {
    const scope = {tenant_id: IDS.tenant, election_event_id: IDS.event, election_id: IDS.election}
    const status = {
        is_published: true,
        voting_status: "OPEN",
        kiosk_voting_status: "OPEN",
        early_voting_status: "CLOSED",
        telephone_voting_status: "CLOSED",
    }
    const eventPresentation = {
        i18n: {en: {name: "Community Election"}},
        language_conf: {default_language_code: "en", enabled_language_codes: ["en", "es"]},
        show_user_profile: false,
        skip_election_list: false,
        redirect_finish_url: finishUrl || null,
        logo_url: null,
        materials: {policy: "off"},
        delegated_voting_policy: "disabled",
    }
    const electionPresentation = {
        i18n: {en: {name: "Community Council", description: "Choose your council representative."}},
        security_confirmation_policy: "none",
        cast_vote_confirm: false,
        cast_vote_gold_level: gold ? "gold-level" : "no-gold-level",
        audit_button_cfg: "show",
        consolidated_report_policy: "do-not-generate",
        contests_order: "custom",
        start_screen_title_policy: "election",
    }
    const ballot = {
        ...scope,
        id: IDS.style,
        area_id: IDS.area,
        description: "Community Council",
        area_presentation: {allow_early_voting: "no_early_voting"},
        public_key: {public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4", is_demo: demo},
        election_event_presentation: eventPresentation,
        election_presentation: electionPresentation,
        contests: [
            {
                ...scope,
                id: IDS.contest,
                name: "Council representative",
                description: "Choose one representative.",
                max_votes: 1,
                min_votes: 1,
                winning_candidates_num: 1,
                voting_type: "first-past-the-post",
                counting_algorithm: "plurality-at-large",
                is_encrypted: true,
                presentation: {candidates_order: "custom", invalid_vote_policy: "not-allowed"},
                candidates: [
                    {id: IDS.alice, name: "Alice Example", presentation: {sort_order: 0}},
                    {id: IDS.bob, name: "Bob Example", presentation: {sort_order: 1}},
                ].map((candidate) => ({...scope, contest_id: IDS.contest, ...candidate})),
            },
        ],
    }
    const event = {
        id: IDS.event,
        tenant_id: IDS.tenant,
        description: "Community Election",
        presentation: eventPresentation,
        status,
    }
    const election = {
        ...scope,
        id: IDS.election,
        description: "Community Council",
        presentation: electionPresentation,
        status,
        num_allowed_revotes: 0,
        voting_channels: {online: true, kiosk: true, early_voting: false, telephone: false},
        is_consolidated_ballot_encoding: false,
        spoil_ballot_option: true,
        annotations: {},
        labels: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    const style = {
        ...scope,
        id: IDS.style,
        area_id: IDS.area,
        ballot_eml: JSON.stringify(ballot),
        ballot_signature: null,
        annotations: {},
        labels: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    const summary = {
        id: IDS.style,
        area_presentation: ballot.area_presentation,
        election_dates: {first_started_at: "2026-01-01T00:00:00Z"},
    }
    return {ballot, event, election, style, summary}
}
