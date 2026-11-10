/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import "@testing-library/jest-dom"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {ApolloClient, ApolloLink, ApolloProvider, InMemoryCache, Observable} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {
    RequesterSigning,
    SigningAction,
    SigningRequirement,
    type ISigningRule,
} from "@/lib/signing/types"
import {RuleDrawer} from "./RuleDrawer"
import {signaturesAccess} from "./signingSettings"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../../ui-core/src/utils/typechecks"),
    ...jest.requireActual("../../../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../../../ui-core/src/services/eventTimeZones"),
}))
jest.mock("react-admin", () => ({useGetList: () => ({data: []}), useNotify: () => jest.fn()}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))

/** Someone who edits the signing rules, but not the roles. */
const access = signaturesAccess((permission) =>
    [IPermissions.SIGNING_RULES_READ, IPermissions.SIGNING_RULES_WRITE].includes(permission)
)

const ruleOf = (action: SigningAction, signatures: number, revision: number): ISigningRule => ({
    action,
    requirement: SigningRequirement.Required,
    signatures,
    requester_signing: RequesterSigning.NotAllowed,
    expires_minutes: 60,
    revision,
})

const SAVED = {
    data: {
        signingPutRule: {
            revision: 1,
            cancelled: [],
            rule: {},
            short_posts: [],
            warnings: [],
            applies: null,
            outcome_changes: null,
        },
    },
}

/** The server's answer to another number for an action whose number is saved. */
const REFUSED_OUT_OF_RANGE = {
    errors: [
        {
            message:
                "The number of signatures of this action is set: it can't change, and the action keeps needing signatures.",
            extensions: {code: "invalid", reason: "out-of-range"},
        },
    ],
}

/** Renders the drawer against a server that records each save and answers `answer`. */
const renderDrawer = (rule: ISigningRule, answer: object = SAVED) => {
    const sent: Array<Record<string, unknown>> = []
    const client = new ApolloClient({
        cache: new InMemoryCache({addTypename: false}),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    if (operation.operationName === "SigningPutRule") {
                        sent.push(operation.variables)
                    }
                    observer.next(answer)
                    observer.complete()
                })
        ),
    })
    render(
        <ApolloProvider client={client}>
            <RuleDrawer
                electionEventId="event"
                rule={rule}
                capacity={undefined}
                access={access}
                onClose={jest.fn()}
                onSaved={jest.fn()}
            />
        </ApolloProvider>
    )
    return sent
}

const signaturesNeeded = () =>
    screen.getByRole("spinbutton", {name: "signing.rule.signaturesNeeded"})
const needsSignatures = () => screen.getByRole("switch", {name: "signing.rule.needsSignatures"})
const save = () => fireEvent.click(screen.getByRole("button", {name: "signing.rule.save"}))

describe("the number of signatures in the rule drawer", () => {
    it("is fixed once a mandatory action has saved it", () => {
        renderDrawer(ruleOf(SigningAction.StartTally, 3, 2))
        expect(signaturesNeeded()).toBeDisabled()
        expect(signaturesNeeded()).toHaveValue(3)
        expect(screen.getByText("signing.rule.signaturesFixed")).toBeVisible()
    })

    it("can still be set while a mandatory action has none", async () => {
        const sent = renderDrawer(ruleOf(SigningAction.StartTally, 0, 0))
        expect(signaturesNeeded()).toBeEnabled()
        expect(screen.queryByText("signing.rule.signaturesFixed")).toBeNull()
        fireEvent.change(signaturesNeeded(), {target: {value: "2"}})
        save()
        await waitFor(() => expect(sent).toHaveLength(1))
        expect(sent[0]).toEqual(
            expect.objectContaining({action: "start-tally", signatures: 2, expected_revision: 0})
        )
    })

    it("is fixed once an action that needs signatures has saved it, and stays needing them", () => {
        renderDrawer(ruleOf(SigningAction.GenerateReports, 3, 5))
        expect(signaturesNeeded()).toBeDisabled()
        expect(signaturesNeeded()).toHaveValue(3)
        expect(screen.getByText("signing.rule.signaturesFixed")).toBeVisible()
        expect(needsSignatures()).toBeChecked()
        expect(needsSignatures()).toBeDisabled()
    })

    it("can be set for an action that needs no signatures yet", async () => {
        const sent = renderDrawer({
            ...ruleOf(SigningAction.GenerateReports, 1, 5),
            requirement: SigningRequirement.NotRequired,
        })
        expect(needsSignatures()).toBeEnabled()
        fireEvent.click(needsSignatures())
        expect(signaturesNeeded()).toBeEnabled()
        fireEvent.change(signaturesNeeded(), {target: {value: "2"}})
        save()
        await waitFor(() => expect(sent).toHaveLength(1))
        expect(sent[0]).toEqual(
            expect.objectContaining({
                requirement: SigningRequirement.Required,
                signatures: 2,
                expected_revision: 5,
            })
        )
    })

    it("says why the server refuses a fixed number", async () => {
        const sent = renderDrawer(ruleOf(SigningAction.StartTally, 3, 2), REFUSED_OUT_OF_RANGE)
        // Another field changes; the number is sent as saved.
        fireEvent.click(screen.getByRole("checkbox", {name: "signing.rule.requesterSigning"}))
        save()
        expect(await screen.findByText("signing.errors.signaturesFixed")).toBeVisible()
        expect(screen.queryByText("signing.errors.invalid")).toBeNull()
        expect(sent[0]).toEqual(expect.objectContaining({signatures: 3, expected_revision: 2}))
    })

    it("keeps the general refusal for a number that isn't fixed", async () => {
        renderDrawer(ruleOf(SigningAction.StartTally, 0, 0), REFUSED_OUT_OF_RANGE)
        fireEvent.change(signaturesNeeded(), {target: {value: "2"}})
        save()
        expect(await screen.findByText("signing.errors.invalid")).toBeVisible()
    })
})
