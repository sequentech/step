// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {Stack} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {COMPLETE_DELIVERY, PACKAGE_SECTIONS, type PackageRequirements} from "./configurationPackage"
import {RequirementsPicker, RequirementsSummary, WizardSteps} from "./PackageHub"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Settings/RequirementsPicker",
    component: RequirementsPicker,
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
} satisfies WidgetMeta<Record<string, never>>
export default meta

const RequirementsStep: React.FC = () => {
    const [requirements, setRequirements] = useState<PackageRequirements>(COMPLETE_DELIVERY)
    return (
        <AdminStoryProvider boundary={boundary}>
            <Stack spacing={2} sx={{p: 2, maxWidth: 640}}>
                <WizardSteps current={0} />
                <span data-testid="story-requirements-summary">
                    <RequirementsSummary requirements={requirements} />
                </span>
                <RequirementsPicker
                    requirements={requirements}
                    onToggleSection={(section) =>
                        setRequirements({
                            ...requirements,
                            sections: requirements.sections.includes(section)
                                ? requirements.sections.filter((s) => s !== section)
                                : PACKAGE_SECTIONS.filter(
                                      (s) => s === section || requirements.sections.includes(s)
                                  ),
                        })
                    }
                    onCensus={(census) => setRequirements({...requirements, census})}
                />
            </Stack>
        </AdminStoryProvider>
    )
}

export const UploadRequirements: StoryObj = {
    render: () => <RequirementsStep />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByTestId("package-wizard-steps").querySelector('[aria-current="step"]')
        ).toHaveTextContent(i18n.t("configurationPackage.hub.stages.package"))
        // Messaging accounts is opt-in by default; selecting it is one click.
        const messaging = canvas.getByRole("checkbox", {name: "messaging-accounts"})
        await expect(messaging).not.toBeChecked()
        await userEvent.click(messaging)
        await expect(messaging).toBeChecked()
        await expect(canvas.getByTestId("story-requirements-summary")).toHaveTextContent("7")
    },
}
