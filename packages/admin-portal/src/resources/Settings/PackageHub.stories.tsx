// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The configuration package layout pieces, fed with the platform's own record
// shapes (configurationPackage.fixture.ts) through the page's view model.

import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {Box, Button, Stack} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {atViewport} from "../../../../test-support/storybook/viewport"
import {
    COMPLETE_DELIVERY,
    PACKAGE_SECTIONS,
    readApplication,
    type PackageRequirements,
} from "./configurationPackage"
import {record, refusedPreview} from "./configurationPackage.fixture"
import type {PackageImportPhase} from "./PackageImportWorkflow"
import {
    type HubTab,
    PackageRail,
    PackageStatusHeader,
    RequirementsPicker,
    RequirementsSummary,
    RunStages,
    SectionBadgeChip,
    TenantReceipt,
    WizardSteps,
} from "./PackageHub"
import {headerActions, lastAppliedReceipt, orderedSectionBadges, runStages} from "./packageOverview"

type Run = "previewed" | "blocked" | "applied" | "failed"

interface Scenario {
    phase: PackageImportPhase
    run: Run
    mode: "preview" | "apply"
    trustLabel: string
}

const annotations = (run: Run) =>
    run === "blocked"
        ? refusedPreview(["Unknown role: synthetic-reviewer"], {missingTrustees: true})
        : record(run)

const HISTORY = [
    {
        id: "synthetic-applied-task",
        created_at: "2026-11-24T14:02:00Z",
        execution_status: "SUCCESS",
        executed_by_user: "story-operator",
        annotations: record("applied"),
    },
]

const Hub: React.FC<Scenario> = ({phase, run, mode, trustLabel}) => {
    const [tab, setTab] = useState<HubTab>("overview")
    const rows = orderedSectionBadges(readApplication(annotations(run)), false)
    const actions = headerActions(phase)
    const label = (action: string) => i18n.t(`configurationPackage.hub.actions.${action}`)
    return (
        <AdminStoryProvider>
            <Stack spacing={1.5} sx={{p: 1}}>
                <PackageStatusHeader
                    phase={phase}
                    trust={{label: trustLabel, detail: "7f3a9c21 … 0b6ec21e"}}
                    onTrust={() => setTab("trust")}
                    actions={
                        <>
                            {actions.secondary && (
                                <Button variant="outlined">
                                    {label(
                                        actions.secondary === "upload"
                                            ? "importAnother"
                                            : actions.secondary
                                    )}
                                </Button>
                            )}
                            <Button variant="contained">
                                {label(
                                    actions.primary === "upload" ? "importPackage" : actions.primary
                                )}
                            </Button>
                        </>
                    }
                    menu={[{label: label("history"), onClick: () => setTab("history")}]}
                />
                <Box
                    sx={{
                        display: "flex",
                        flexDirection: {xs: "column", md: "row"},
                        gap: 1.5,
                        alignItems: {md: "flex-start"},
                    }}
                >
                    <PackageRail tab={tab} onTab={setTab} sections={rows} trustBlocked={false} />
                    <Box
                        sx={{
                            flex: "1 1 auto",
                            minWidth: 0,
                            display: "grid",
                            gridTemplateColumns: {
                                xs: "minmax(0, 1fr)",
                                lg: "minmax(0, 1.3fr) minmax(300px, 1fr)",
                            },
                            gap: 1.5,
                            alignItems: "start",
                        }}
                    >
                        <Stack spacing={0.5} data-testid="story-section-badges">
                            {rows.map(({section, badge}) => (
                                <Stack
                                    key={section}
                                    direction="row"
                                    sx={{justifyContent: "space-between"}}
                                >
                                    <span>
                                        {i18n.t(`configurationPackage.sections.${section}`)}
                                    </span>
                                    <SectionBadgeChip badge={badge} />
                                </Stack>
                            ))}
                        </Stack>
                        <Stack spacing={1.5}>
                            <RunStages stages={runStages(phase, mode)} />
                            <TenantReceipt
                                receipt={lastAppliedReceipt(HISTORY)}
                                onHistory={() => setTab("history")}
                            />
                        </Stack>
                    </Box>
                </Box>
            </Stack>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Settings/PackageHub",
    component: PackageStatusHeader,
    args: {phase: "review", run: "previewed", mode: "preview", trustLabel: "Roster active"},
    parameters: {
        widgets: [
            "PackageStatusHeader",
            "PackageRail",
            "BadgeDot",
            "RunStages",
            "SectionBadgeChip",
            "TenantReceipt",
        ],
    },
    render: (args) => <Hub {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const badgeOf = (canvasElement: HTMLElement, section: string) =>
    within(canvasElement)
        .getByTestId(`package-rail-${section}`)
        .querySelector("[data-badge]")
        ?.getAttribute("data-badge")

export const PreviewReady: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("button", {
                name: i18n.t("configurationPackage.hub.actions.reviewAndApply"),
            })
        ).toBeVisible()
        await expect(badgeOf(canvasElement, "tenant-configuration")).toBe("willChange")
        await expect(canvas.getByTestId("package-receipt")).toHaveTextContent(
            "synthetic-applied-task"
        )
    },
}

export const PreviewBlocked: Story = {
    args: {phase: "previewBlocked", run: "blocked"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("button", {
                name: i18n.t("configurationPackage.hub.actions.importFixed"),
            })
        ).toBeVisible()
        await expect(
            canvas.getByRole("button", {
                name: i18n.t("configurationPackage.hub.actions.changeRequirements"),
            })
        ).toBeVisible()
        await expect(
            canvas.queryByRole("button", {
                name: i18n.t("configurationPackage.hub.actions.reviewAndApply"),
            })
        ).not.toBeInTheDocument()
        await expect(badgeOf(canvasElement, "tenant-configuration")).toBe("blocked")
        await expect(badgeOf(canvasElement, "trustees")).toBe("missing")
    },
}

export const Applied: Story = {
    args: {phase: "complete", run: "applied", mode: "apply"},
    play: async ({canvasElement}) => {
        const stages = within(canvasElement).getByTestId("package-run-stages")
        await expect(
            Array.from(stages.querySelectorAll("[data-state]")).map((stage) =>
                stage.getAttribute("data-state")
            )
        ).toEqual(["done", "done", "done", "done"])
        await userEvent.click(within(canvasElement).getByTestId("package-rail-history"))
        await expect(within(canvasElement).getByTestId("package-rail-history")).toHaveAttribute(
            "aria-current",
            "page"
        )
    },
}

export const StoppedOnFailure: Story = {
    args: {phase: "attention", run: "failed", mode: "apply"},
    play: async ({canvasElement}) => {
        await expect(badgeOf(canvasElement, "tenant-configuration")).toBe("applied")
        await expect(badgeOf(canvasElement, "election-event")).toBe("failed")
    },
}

export const PhoneBlocked: Story = {
    ...atViewport(390),
    args: {phase: "previewBlocked", run: "blocked"},
    play: async ({canvasElement}) => {
        const rail = within(canvasElement).getByTestId("package-rail")
        // The rail becomes a horizontal strip above the panels.
        await expect(rail.getBoundingClientRect().width).toBeLessThanOrEqual(390)
        await expect(
            within(canvasElement).getByRole("button", {
                name: i18n.t("configurationPackage.hub.actions.importFixed"),
            })
        ).toBeVisible()
    },
}

const RequirementsStep: React.FC = () => {
    const [requirements, setRequirements] = useState<PackageRequirements>(COMPLETE_DELIVERY)
    return (
        <AdminStoryProvider>
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

export const UploadRequirements: Story = {
    parameters: {widgets: ["WizardSteps", "RequirementsSummary", "RequirementsPicker"]},
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
