// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Layout pieces of the configuration package page: the compact status header,
// the inner rail (Overview, the seven sections in apply order, Signing trust,
// History), section badges, the run's stages and the tenant receipt.
// Presentation only: every action is passed in by the page, which keeps its gates.

import React, {useId, useState} from "react"
import {
    Box,
    Button,
    ButtonBase,
    Checkbox,
    Chip,
    Divider,
    FormControlLabel,
    IconButton,
    List,
    ListItemButton,
    ListSubheader,
    Menu,
    MenuItem,
    Paper,
    Stack,
    Typography,
} from "@mui/material"
import type {ChipProps} from "@mui/material"
import MoreHorizIcon from "@mui/icons-material/MoreHoriz"
import {useTranslation} from "react-i18next"
import {
    PACKAGE_SECTIONS,
    type PackageRequirements,
    type PackageSection,
} from "./configurationPackage"
import type {PackageImportPhase} from "./PackageImportWorkflow"
import type {AppliedReceipt, RunStage, SectionBadge, SectionRow} from "./packageOverview"

export type HubTab = "overview" | PackageSection | "trust" | "history"

export const isSectionTab = (tab: HubTab): tab is PackageSection =>
    (PACKAGE_SECTIONS as readonly string[]).includes(tab)

const BADGE_COLOR: Record<SectionBadge, ChipProps["color"]> = {
    applied: "success",
    unchanged: "default",
    willChange: "info",
    blocked: "warning",
    missing: "warning",
    notInPackage: "default",
    failed: "error",
    notConfirmed: "warning",
    applying: "info",
    waiting: "default",
    notStarted: "default",
    notRun: "default",
}

export const SectionBadgeChip: React.FC<{badge: SectionBadge}> = ({badge}) => {
    const {t} = useTranslation()
    return (
        <Chip
            size="small"
            color={BADGE_COLOR[badge]}
            variant={badge === "applied" || badge === "failed" ? "filled" : "outlined"}
            label={String(t(`configurationPackage.hub.badge.${badge}`))}
            data-badge={badge}
            sx={{"blockSize": "auto", "& .MuiChip-label": {whiteSpace: "normal"}}}
        />
    )
}

const PHASE_COLOR: Record<PackageImportPhase, ChipProps["color"]> = {
    waitingForTrust: "default",
    trustBlocked: "warning",
    choose: "default",
    starting: "info",
    previewing: "info",
    previewBlocked: "warning",
    previewAgain: "warning",
    review: "info",
    applying: "info",
    complete: "success",
    attention: "error",
}

/** Compact header: the phase, the signing trust summary and the page's actions. */
export const PackageStatusHeader: React.FC<{
    phase: PackageImportPhase
    trust: {label: string; detail?: string}
    /** Id for the trust line, so a disabled import can point at the reason. */
    trustTextId?: string
    onTrust: () => void
    actions: React.ReactNode
    menu: {label: string; onClick: () => void; disabled?: boolean; testId?: string}[]
    children?: React.ReactNode
}> = ({phase, trust, trustTextId, onTrust, actions, menu, children}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const menuId = useId()
    const [anchor, setAnchor] = useState<HTMLElement | null>(null)
    const color = PHASE_COLOR[phase]
    return (
        <Paper
            component="section"
            variant="outlined"
            aria-labelledby={titleId}
            data-testid="package-workflow"
            data-phase={phase}
            sx={{
                p: 1.5,
                pl: 2,
                display: "flex",
                flexWrap: {xs: "wrap", md: "nowrap"},
                alignItems: "center",
                columnGap: 2,
                rowGap: 1.5,
                borderInlineStart: 4,
                borderInlineStartColor: color === "default" ? "divider" : `${color as string}.main`,
            }}
        >
            <Box sx={{flex: "1 1 320px", minWidth: 0}}>
                <Typography id={titleId} variant="subtitle1" component="h3" sx={{fontWeight: 500}}>
                    {t(`configurationPackage.workflow.phases.${phase}.title`)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                    {t(`configurationPackage.workflow.phases.${phase}.help`)}
                </Typography>
            </Box>
            <Divider orientation="vertical" flexItem sx={{display: {xs: "none", md: "block"}}} />
            {/* A neutral outlined tile, not a themed Button: the admin theme fills those navy. */}
            <ButtonBase
                onClick={onTrust}
                data-testid="package-trust-summary"
                aria-label={`${t("configurationPackage.hub.trustLabel")}: ${trust.label}`}
                sx={{
                    "flex": "0 0 auto",
                    "display": {xs: "none", md: "flex"},
                    "flexDirection": "column",
                    "alignItems": "flex-start",
                    "textAlign": "start",
                    "minWidth": 200,
                    "px": 1.5,
                    "py": 0.75,
                    "border": 1,
                    "borderColor": "divider",
                    "borderRadius": 1,
                    "bgcolor": "background.paper",
                    "color": "text.primary",
                    "&:hover": {bgcolor: "action.hover"},
                    "&.Mui-focusVisible": {
                        outline: (theme) => `2px solid ${theme.palette.primary.main}`,
                        outlineOffset: 2,
                    },
                }}
            >
                <Typography variant="overline" color="text.secondary" sx={{lineHeight: 1.6}}>
                    {t("configurationPackage.hub.trustLabel")}
                </Typography>
                <Typography id={trustTextId} variant="body2" color="text.primary">
                    {trust.label}
                </Typography>
                {trust.detail && (
                    <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{fontFamily: "monospace"}}
                    >
                        {trust.detail}
                    </Typography>
                )}
            </ButtonBase>
            <Divider orientation="vertical" flexItem sx={{display: {xs: "none", md: "block"}}} />
            <Stack
                direction="row"
                spacing={1}
                sx={{
                    flex: {xs: "1 1 100%", md: "0 0 auto"},
                    alignItems: "center",
                    justifyContent: "flex-end",
                    flexWrap: "wrap",
                    rowGap: 1,
                }}
            >
                {actions}
                <IconButton
                    aria-label={String(t("configurationPackage.hub.actions.more"))}
                    aria-haspopup="menu"
                    aria-controls={anchor ? menuId : undefined}
                    aria-expanded={anchor ? "true" : "false"}
                    onClick={(event) => setAnchor(event.currentTarget)}
                    sx={{border: 1, borderColor: "divider", borderRadius: 1}}
                >
                    <MoreHorizIcon fontSize="small" />
                </IconButton>
                <Menu id={menuId} anchorEl={anchor} open={!!anchor} onClose={() => setAnchor(null)}>
                    {menu.map((item) => (
                        <MenuItem
                            key={item.label}
                            disabled={item.disabled}
                            data-testid={item.testId}
                            onClick={() => {
                                setAnchor(null)
                                item.onClick()
                            }}
                        >
                            {item.label}
                        </MenuItem>
                    ))}
                </Menu>
            </Stack>
            {children && <Box sx={{flex: "1 1 100%", minWidth: 0}}>{children}</Box>}
        </Paper>
    )
}

/** Overview, the seven sections in apply order, then Signing trust and History. */
export const PackageRail: React.FC<{
    tab: HubTab
    onTab: (tab: HubTab) => void
    sections: SectionRow[]
    trustBlocked: boolean
}> = ({tab, onTab, sections, trustBlocked}) => {
    const {t} = useTranslation()
    const item = (id: HubTab, label: string, lead: React.ReactNode, trail?: React.ReactNode) => (
        <ListItemButton
            key={id}
            selected={tab === id}
            aria-current={tab === id ? "page" : undefined}
            onClick={() => onTab(id)}
            data-testid={`package-rail-${id}`}
            sx={{
                "borderRadius": 1,
                "gap": 1.25,
                "minHeight": 40,
                "flex": {xs: "0 0 auto", md: "initial"},
                "&.Mui-selected": {
                    boxShadow: (theme) => `inset 3px 0 0 ${theme.palette.primary.main}`,
                },
            }}
        >
            <Box
                component="span"
                aria-hidden="true"
                sx={{
                    width: 22,
                    height: 22,
                    flex: "0 0 22px",
                    borderRadius: "50%",
                    border: lead ? 1 : 0,
                    borderColor: "divider",
                    display: {xs: lead ? "inline-flex" : "none", md: "inline-flex"},
                    alignItems: "center",
                    justifyContent: "center",
                    typography: "caption",
                    color: "text.secondary",
                }}
            >
                {lead}
            </Box>
            <Typography
                variant="body2"
                component="span"
                sx={{flex: "1 1 auto", minWidth: 0}}
                noWrap
            >
                {label}
            </Typography>
            {trail}
        </ListItemButton>
    )
    return (
        <Paper
            component="nav"
            variant="outlined"
            aria-label={String(t("configurationPackage.hub.rail.label"))}
            data-testid="package-rail"
            sx={{
                flex: {md: "0 0 248px"},
                // Stretched on phones, so its own horizontal scroll applies instead of
                // the strip widening the page; top-aligned beside the panels on wide screens.
                alignSelf: {xs: "stretch", md: "start"},
                p: 0.75,
                minWidth: 0,
                maxWidth: "100%",
                overflowX: {xs: "auto", md: "visible"},
            }}
        >
            <List
                dense
                disablePadding
                sx={{display: "flex", flexDirection: {xs: "row", md: "column"}, gap: 0.25}}
            >
                {item("overview", String(t("configurationPackage.hub.rail.overview")), null)}
                <ListSubheader
                    disableSticky
                    sx={{
                        display: {xs: "none", md: "block"},
                        lineHeight: 2.5,
                        typography: "overline",
                        color: "text.secondary",
                    }}
                >
                    {t("configurationPackage.hub.rail.sections")}
                </ListSubheader>
                {sections.map(({section, badge}, index) =>
                    item(
                        section,
                        String(t(`configurationPackage.sections.${section}`)),
                        index + 1,
                        <Box
                            component="span"
                            sx={{display: {xs: "none", md: "inline-flex"}}}
                            title={String(t(`configurationPackage.hub.badge.${badge}`))}
                        >
                            <BadgeDot badge={badge} />
                        </Box>
                    )
                )}
                <Divider sx={{my: 0.75, display: {xs: "none", md: "block"}}} />
                {item(
                    "trust",
                    String(t("configurationPackage.hub.rail.trust")),
                    null,
                    trustBlocked ? (
                        <Box component="span" sx={{display: "inline-flex"}}>
                            <BadgeDot badge="blocked" />
                        </Box>
                    ) : undefined
                )}
                {item("history", String(t("configurationPackage.hub.rail.history")), null)}
            </List>
        </Paper>
    )
}

/** A small status mark; the chip next to it, or its title, carries the words. */
const BadgeDot: React.FC<{badge: SectionBadge}> = ({badge}) => {
    const color = BADGE_COLOR[badge]
    return (
        <Box
            component="span"
            aria-hidden="true"
            data-badge={badge}
            sx={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                bgcolor: color === "default" ? "transparent" : `${color as string}.main`,
                border: color === "default" ? 1.5 : 0,
                borderColor: "text.disabled",
            }}
        />
    )
}

const STAGE_MARK: Record<RunStage["state"], string> = {
    done: "✓",
    current: "•",
    running: "…",
    blocked: "!",
    stale: "!",
    attention: "!",
    pending: "",
}

/** Package → Preview → Review → Apply for the run being shown. */
export const RunStages: React.FC<{stages: RunStage[]}> = ({stages}) => {
    const {t} = useTranslation()
    return (
        <Box
            component="ol"
            aria-label={String(t("configurationPackage.hub.stagesLabel"))}
            data-testid="package-run-stages"
            sx={{
                display: "grid",
                gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
                listStyle: "none",
                m: 0,
                p: 0,
                gap: 1,
            }}
        >
            {stages.map(({step, state}) => {
                const tone =
                    state === "done"
                        ? "success.main"
                        : state === "attention"
                          ? "error.main"
                          : state === "blocked" || state === "stale"
                            ? "warning.main"
                            : state === "current" || state === "running"
                              ? "info.main"
                              : "divider"
                return (
                    <Box
                        component="li"
                        key={step}
                        aria-current={
                            state === "current" || state === "running" ? "step" : undefined
                        }
                        data-state={state}
                        sx={{display: "flex", flexDirection: "column", gap: 0.5, minWidth: 0}}
                    >
                        <Box
                            aria-hidden="true"
                            sx={{
                                height: 4,
                                borderRadius: 2,
                                bgcolor: tone,
                            }}
                        />
                        <Typography variant="body2" sx={{fontWeight: 500}} noWrap>
                            {STAGE_MARK[state] ? `${STAGE_MARK[state]} ` : ""}
                            {t(`configurationPackage.hub.stages.${step}`)}
                        </Typography>
                        <Typography variant="caption" color="text.secondary" noWrap>
                            {t(`configurationPackage.hub.stageState.${state}`)}
                        </Typography>
                    </Box>
                )
            })}
        </Box>
    )
}

/** What the platform recorded as applied on this tenant, separate from the run. */
export const TenantReceipt: React.FC<{
    receipt: AppliedReceipt | null | undefined
    onHistory: () => void
}> = ({receipt, onHistory}) => {
    const {t} = useTranslation()
    const headingId = useId()
    const rows: [string, string, boolean?][] = receipt
        ? [
              [
                  String(t("configurationPackage.hub.receipt.sha")),
                  receipt.packageSha256 ?? "?",
                  true,
              ],
              // The task row's created_at: when the run started, not when it finished.
              [String(t("configurationPackage.hub.historyStarted")), receipt.startedAt],
              ...(receipt.appliedBy
                  ? ([[String(t("configurationPackage.hub.receipt.by")), receipt.appliedBy]] as [
                        string,
                        string,
                    ][])
                  : []),
              [String(t("configurationPackage.hub.receipt.task")), receipt.taskId, true],
          ]
        : []
    return (
        <Paper
            component="section"
            variant="outlined"
            aria-labelledby={headingId}
            data-testid="package-receipt"
            sx={{p: 2, minWidth: 0}}
        >
            <Stack direction="row" sx={{alignItems: "center", justifyContent: "space-between"}}>
                <Typography id={headingId} variant="subtitle2" component="h3">
                    {t("configurationPackage.hub.receipt.title")}
                </Typography>
                <Button size="small" variant="text" onClick={onHistory}>
                    {t("configurationPackage.hub.rail.history")}
                </Button>
            </Stack>
            {receipt === undefined ? (
                <Typography variant="body2" color="text.secondary">
                    {t("configurationPackage.hub.receipt.loading")}
                </Typography>
            ) : receipt === null ? (
                <Typography variant="body2" color="text.secondary">
                    {t("configurationPackage.hub.receipt.none")}
                </Typography>
            ) : (
                <Box
                    component="dl"
                    sx={{
                        display: "grid",
                        gridTemplateColumns: "max-content minmax(0, 1fr)",
                        columnGap: 2,
                        rowGap: 0.75,
                        m: 0,
                        mt: 1,
                    }}
                >
                    {rows.map(([label, value, mono]) => (
                        <React.Fragment key={label}>
                            <Typography component="dt" variant="body2" color="text.secondary">
                                {label}
                            </Typography>
                            <Typography
                                component="dd"
                                variant="body2"
                                title={value}
                                sx={{
                                    m: 0,
                                    minWidth: 0,
                                    overflow: "hidden",
                                    textOverflow: "ellipsis",
                                    whiteSpace: "nowrap",
                                    fontFamily: mono ? "monospace" : undefined,
                                }}
                            >
                                {value}
                            </Typography>
                        </React.Fragment>
                    ))}
                </Box>
            )}
        </Paper>
    )
}

/** One line summary of the requirements, e.g. "6 of 7 sections required · Census required". */
export const RequirementsSummary: React.FC<{requirements: PackageRequirements}> = ({
    requirements,
}) => {
    const {t} = useTranslation()
    return (
        <>
            {t("configurationPackage.workflow.optionsSummary", {
                selected: requirements.sections.length,
                available: PACKAGE_SECTIONS.length,
            })}
            {" · "}
            {t(
                requirements.census === "required"
                    ? "configurationPackage.workflow.censusRequired"
                    : "configurationPackage.workflow.censusOptional"
            )}
        </>
    )
}

/**
 * The required sections, in apply order, and the census. Changing them is the
 * page's changeRequirements, which also clears the current preview.
 */
export const RequirementsPicker: React.FC<{
    requirements: PackageRequirements
    onToggleSection: (section: PackageSection) => void
    onCensus: (census: PackageRequirements["census"]) => void
    disabled?: boolean
    testId?: string
}> = ({requirements, onToggleSection, onCensus, disabled, testId}) => {
    const {t} = useTranslation()
    const headingId = useId()
    return (
        <Box role="group" aria-labelledby={headingId} data-testid={testId} sx={{display: "grid"}}>
            <Typography id={headingId} variant="subtitle1" component="h4">
                {t("configurationPackage.requirements")}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{mb: 0.5}}>
                {t("configurationPackage.hub.requirementsHelp")}
            </Typography>
            {PACKAGE_SECTIONS.map((section, index) => (
                <FormControlLabel
                    key={section}
                    disabled={disabled}
                    control={
                        <Checkbox
                            checked={requirements.sections.includes(section)}
                            onChange={() => onToggleSection(section)}
                            inputProps={{"aria-label": section}}
                        />
                    }
                    label={`${index + 1}. ${t(`configurationPackage.sections.${section}`)}`}
                />
            ))}
            <FormControlLabel
                disabled={disabled}
                control={
                    <Checkbox
                        checked={requirements.census === "required"}
                        onChange={(event) =>
                            onCensus(event.target.checked ? "required" : "optional")
                        }
                        inputProps={{"aria-label": "census"}}
                    />
                }
                label={String(t("configurationPackage.censusRequired"))}
            />
        </Box>
    )
}

/** Upload → Preview → Review → Apply, with the current step marked. */
export const WizardSteps: React.FC<{current: number}> = ({current}) => {
    const {t} = useTranslation()
    const steps = ["package", "preview", "review", "apply"] as const
    return (
        <Box
            component="ol"
            aria-label={String(t("configurationPackage.hub.stagesLabel"))}
            data-testid="package-wizard-steps"
            sx={{display: "flex", flexWrap: "wrap", gap: 2, listStyle: "none", m: 0, p: 0}}
        >
            {steps.map((step, index) => (
                <Box
                    component="li"
                    key={step}
                    aria-current={index === current ? "step" : undefined}
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        gap: 0.75,
                        typography: "body2",
                        fontWeight: index === current ? 500 : 400,
                        color: index === current ? "text.primary" : "text.secondary",
                    }}
                >
                    <Box
                        component="span"
                        aria-hidden="true"
                        sx={{
                            width: 22,
                            height: 22,
                            borderRadius: "50%",
                            border: 1,
                            borderColor: index === current ? "primary.main" : "divider",
                            bgcolor: index === current ? "primary.main" : "transparent",
                            color: index === current ? "primary.contrastText" : "text.secondary",
                            display: "inline-flex",
                            alignItems: "center",
                            justifyContent: "center",
                            typography: "caption",
                        }}
                    >
                        {index < current ? "✓" : index + 1}
                    </Box>
                    {t(`configurationPackage.hub.stages.${step}`)}
                </Box>
            ))}
        </Box>
    )
}
