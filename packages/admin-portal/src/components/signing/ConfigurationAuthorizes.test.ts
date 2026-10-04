// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {testI18n} from "@/components/timezones/__fixtures__/testI18n"
import type {TFunction} from "i18next"
import {EInitializeReportPolicy as ReportPolicy} from "../../../../ui-core/src/types/ElectionPresentation"
import {
    EInitializationScope as Scope,
    EUnsignedScheduledClosePolicy as Close,
} from "../../../../ui-core/src/types/ElectionEventPresentation"
import {
    ConfigurationAuthorizes,
    approvalTarget,
    configurationDiff,
    postChannelsOf,
    previousApproval,
    snapshotOfSubject,
} from "./ConfigurationAuthorizes"
import {EPolicyChange} from "@/components/election-event/lifecyclePolicyChange"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionPresentation"),
}))
jest.mock("react-admin", () => ({useGetList: () => ({data: mockPosts})}))
jest.mock("@apollo/client", () => ({
    gql: (s: TemplateStringsArray) => s.join(""),
    useQuery: () => ({loading: false}),
}))
const mockI18n = testI18n()
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockI18n.t})}))
jest.mock("@/hooks/useAliasRenderer", () => ({
    useAliasRenderer: () => (post: {name: string}) => post.name,
}))
jest.mock("@/components/timezones/timeZoneService", () => ({useTimeZoneService: () => ({})}))
jest.mock("@/components/timezones/useTimeZoneContext", () => ({useTimeZoneContext: () => ({})}))
const mockPosts = [
    {
        id: "madrid",
        name: "Madrid Post",
        presentation: {initialization_report_policy: "not-required"},
    },
]

const t = ((key: string, options?: Record<string, unknown>) =>
    options?.count === undefined ? key : `${key}:${options.count}`) as unknown as TFunction

describe("configurationDiff", () => {
    const base = {
        policies: {initialization_scope: Scope.POST, unsigned_scheduled_close: Close.REFUSE},
        open_voting: {required: true, signatures: 2},
        close_voting: {required: true, signatures: 2},
    }

    it("names nothing when the values are the same", () => {
        expect(configurationDiff(t, base, base)).toEqual([])
    })

    it("names each loosening and tightening with its values", () => {
        const next = {
            policies: {
                initialization_scope: Scope.EVENT,
                unsigned_scheduled_close: Close.RUN_AS_SYSTEM,
            },
            open_voting: {required: false},
            close_voting: {required: true, signatures: 3},
        }
        expect(configurationDiff(t, base, next)).toEqual([
            {
                change: EPolicyChange.TIGHTENS,
                setting: "lifecycle.policies.scope.title",
                before: "lifecycle.policies.scope.post.label",
                after: "lifecycle.policies.scope.event.label",
            },
            {
                change: EPolicyChange.LOOSENS,
                setting: "lifecycle.policies.close.title",
                before: "lifecycle.policies.close.refuse.label",
                after: "lifecycle.policies.close.runAsSystem.label",
            },
            {
                change: EPolicyChange.LOOSENS,
                setting: "lifecycle.authorizes.rule.openSetting",
                before: "lifecycle.authorizes.rule.signatures:2",
                after: "lifecycle.authorizes.rule.none",
            },
            {
                change: EPolicyChange.TIGHTENS,
                setting: "lifecycle.authorizes.rule.closeSetting",
                before: "lifecycle.authorizes.rule.signatures:2",
                after: "lifecycle.authorizes.rule.signatures:3",
            },
        ])
    })

    it("shows a per-Post report policy change even when every other setting is unchanged", () => {
        const previous = {
            ...base,
            initialization_report_policies: {madrid: ReportPolicy.NOT_REQUIRED},
        }
        const next = {
            ...base,
            initialization_report_policies: {madrid: ReportPolicy.REQUIRED},
        }
        expect(configurationDiff(mockI18n.t, previous, next, () => "Madrid Post")).toEqual([
            {
                change: EPolicyChange.TIGHTENS,
                setting: "Madrid Post: Initialize Report Policy",
                before: "Not Required",
                after: "Required",
            },
        ])
        expect(configurationDiff(mockI18n.t, next, base, () => "Madrid Post")).toEqual([
            {
                change: EPolicyChange.LOOSENS,
                setting: "Madrid Post: Initialize Report Policy",
                before: "Required",
                after: "Not Required",
            },
        ])
        expect(configurationDiff(mockI18n.t, base, previous)).toEqual([])
    })

    it("reads missing policies as the defaults", () => {
        expect(configurationDiff(t, {}, {policies: base.policies})).toEqual([])
    })
})

describe("previousApproval", () => {
    const POST = "33333333-3333-4333-8333-000000000001"
    const OTHER = "33333333-3333-4333-8333-000000000002"
    const approval = (id: string, target: string) => ({
        id,
        code: id,
        scope_key: `event-id|approve-configuration|${target}`,
        subject: {},
    })

    it("reads the target from the scope key", () => {
        expect(approvalTarget(`event-id|approve-configuration|${POST}`)).toBe(POST)
        expect(approvalTarget("event-id|approve-configuration|event")).toBeNull()
        expect(approvalTarget(undefined)).toBeNull()
    })

    it("is the newest approval of the Post itself or of the event", () => {
        const newestFirst = [
            approval("other", OTHER),
            approval("post", POST),
            approval("event", "event"),
        ]
        expect(previousApproval(newestFirst, POST)?.id).toBe("post")
        expect(previousApproval(newestFirst.slice(2), POST)?.id).toBe("event")
    })

    it("for the event, only the event's", () => {
        expect(
            previousApproval([approval("post", POST), approval("event", "event")], null)?.id
        ).toBe("event")
        expect(previousApproval([approval("post", POST)], null)).toBeNull()
    })
})

describe("postChannelsOf", () => {
    it("lists the channels each election had when approved", () => {
        expect(postChannelsOf({post_channels: {dubai: ["ONLINE", "KIOSK"], tokyo: []}})).toEqual([
            ["dubai", ["ONLINE", "KIOSK"]],
            ["tokyo", []],
        ])
        expect(postChannelsOf({})).toEqual([])
    })
})

describe("snapshotOfSubject", () => {
    it("reads an older subject without a snapshot as an empty schedule", () => {
        expect(snapshotOfSubject({digest: "x"})).toEqual({
            schedule: [],
            policies: null,
            open_voting: null,
            close_voting: null,
        })
    })
})

describe("signed initialization report requirements", () => {
    it("does not invent report requirements for a legacy subject without the map", () => {
        const html = renderToStaticMarkup(
            React.createElement(ConfigurationAuthorizes, {
                subject: {digest: "legacy"},
                electionEventId: "event",
                requestId: "request",
            })
        )
        expect(snapshotOfSubject({digest: "legacy"}).initialization_report_policies).toBeUndefined()
        expect(html).not.toContain("authorizes-initialization-reports")
        expect(html).not.toContain("remains required")
    })

    it("retains the signed per-Post requirement when the current Post does not require it", () => {
        const subject = {
            policies: {initialization_scope: Scope.POST},
            initialization_report_policies: {madrid: "required", canary: "not-required"},
        }
        expect(snapshotOfSubject(subject).initialization_report_policies).toEqual({
            madrid: "required",
            canary: "not-required",
        })
        const html = renderToStaticMarkup(
            React.createElement(ConfigurationAuthorizes, {
                subject,
                electionEventId: "event",
                requestId: "request",
            })
        )
        expect(html).toContain("Madrid Post: Required")
        expect(html).toContain("canary: Not Required")
        expect(html).toContain("remains required")
    })
})
