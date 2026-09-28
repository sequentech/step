// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {expect, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {useTranslation} from "react-i18next"
import {EGracePeriodPolicy, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {ManagedNumberInput} from "../ManagedNumberInput"
import {ManagedSelectInput} from "../ManagedSelectInput"

/** The election's saved grace period, as the election form edits it. */
export interface GracePeriodScenario {
    /** The saved policy; a new election has none. */
    policy?: EGracePeriodPolicy
    seconds?: number
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>

export async function setUpGracePeriodForm() {
    graphql = graphqlBoundary({}, {schema: true})
    await graphql.ready
}

/** The grace period inputs of the election form: the seconds follow the policy. */
export function GracePeriodForm({policy, seconds, onSubmit}: GracePeriodScenario) {
    const {t} = useTranslation()
    return (
        <AdminStoryProvider boundary={graphql}>
            <SimpleForm
                record={
                    policy
                        ? {presentation: {grace_period_policy: policy, grace_period_secs: seconds}}
                        : {}
                }
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <ManagedSelectInput
                    source="presentation.grace_period_policy"
                    choices={Object.values(EGracePeriodPolicy).map((value) => ({
                        id: value,
                        name: t(`electionScreen.gracePeriodPolicy.${value.toLowerCase()}`),
                    }))}
                    label={t("electionScreen.gracePeriodPolicy.label")}
                    defaultValue={EGracePeriodPolicy.NO_GRACE_PERIOD}
                />
                <ManagedNumberInput
                    source="presentation.grace_period_secs"
                    label={t("electionScreen.gracePeriodPolicy.gracePeriodSecs")}
                    defaultValue={0}
                    sourceToWatch="presentation.grace_period_policy"
                    isDisabled={(selected) => selected === EGracePeriodPolicy.NO_GRACE_PERIOD}
                />
            </SimpleForm>
        </AdminStoryProvider>
    )
}

export const policyName = (policy: EGracePeriodPolicy) =>
    i18n.t(`electionScreen.gracePeriodPolicy.${policy.toLowerCase()}`)

export const policySelect = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {
        name: new RegExp(i18n.t("electionScreen.gracePeriodPolicy.label")),
    })

export const secondsInput = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("spinbutton", {
        name: i18n.t("electionScreen.gracePeriodPolicy.gracePeriodSecs"),
    })

export async function choosePolicy(canvasElement: HTMLElement, policy: EGracePeriodPolicy) {
    await userEvent.click(policySelect(canvasElement))
    await userEvent.click(
        await within(document.body).findByRole("option", {name: policyName(policy)})
    )
    await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull())
}

/** Saves the form and returns the presentation it submitted. */
export async function savedPresentation(
    canvasElement: HTMLElement,
    onSubmit: GracePeriodScenario["onSubmit"]
) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
    return onSubmit.mock.calls[0][0].presentation
}
