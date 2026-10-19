// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useId, useRef} from "react"
import {Box} from "@mui/material"
import Tabs from "@mui/material/Tabs"
import Tab from "@mui/material/Tab"
import {useTheme} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {useTranslation} from "react-i18next"

export enum ESlateBallotTab {
    SLATES = "slates",
    CANDIDATES = "candidates",
}

export interface SlateBallotTabsProps {
    value: ESlateBallotTab
    onChange: (value: ESlateBallotTab) => void
    slates: React.ReactNode
    candidates: React.ReactNode
}

/**
 * Both panels stay mounted. The contests report their validation state from
 * effects, so unmounting them while the voter looks at the slates would lose
 * the checks that run before review.
 */
const TAB_STYLE = {
    "minHeight": "44px",
    "minWidth": 0,
    "padding": {xs: "11px 0 12px", sm: "12px 0 14px"},
    "marginRight": {xs: 0, sm: "22px"},
    "textTransform": "none",
    "overflowWrap": "anywhere",
    "fontSize": {xs: "15px", sm: "16px"},
    "color": "customGrey.dark",
    "&.Mui-selected": {
        color: "brandColor",
        fontWeight: 600,
    },
} as const

export const SlateBallotTabs: React.FC<SlateBallotTabsProps> = ({
    value,
    onChange,
    slates,
    candidates,
}) => {
    const {t} = useTranslation()
    const theme = useTheme()
    const isPhone = useMediaQuery(theme.breakpoints.down("sm"))
    const id = useId()
    const tabId = (tab: ESlateBallotTab) => `${id}-tab-${tab}`
    const panelId = (tab: ESlateBallotTab) => `${id}-panel-${tab}`
    const containerRef = useRef<HTMLDivElement>(null)
    const isFirstRender = useRef(true)

    // A button inside a panel can switch tabs, which hides the button that
    // had focus. The focus then moves to the tab that became selected.
    useEffect(() => {
        if (isFirstRender.current) {
            isFirstRender.current = false
            return
        }
        const selected = containerRef.current?.querySelector<HTMLElement>(
            '[role="tab"][aria-selected="true"]'
        )
        if (selected && document.activeElement !== selected) {
            selected.focus()
        }
    }, [value])

    const panels: Array<[ESlateBallotTab, React.ReactNode]> = [
        [ESlateBallotTab.SLATES, slates],
        [ESlateBallotTab.CANDIDATES, candidates],
    ]

    return (
        <Box className="slate-ballot-tabs-container" ref={containerRef}>
            <Box
                className="slate-ballot-tabs-bar"
                sx={{borderBottom: 1, borderColor: "divider", marginBottom: "24px"}}
            >
                <Tabs
                    className="slate-ballot-tabs"
                    variant={isPhone ? "fullWidth" : "standard"}
                    indicatorColor="primary"
                    textColor="inherit"
                    sx={{
                        "& .MuiTabs-indicator": {
                            height: "3px",
                            backgroundColor: theme.palette.brandColor,
                        },
                    }}
                    aria-label={t("slates.tabs.label")}
                    value={value}
                    onChange={(_event, next: ESlateBallotTab) => onChange(next)}
                >
                    <Tab
                        className="slate-ballot-tab-slates"
                        label={t("slates.tabs.slates")}
                        value={ESlateBallotTab.SLATES}
                        id={tabId(ESlateBallotTab.SLATES)}
                        aria-controls={panelId(ESlateBallotTab.SLATES)}
                        sx={TAB_STYLE}
                    />
                    <Tab
                        className="slate-ballot-tab-candidates"
                        label={t("slates.tabs.candidates")}
                        value={ESlateBallotTab.CANDIDATES}
                        id={tabId(ESlateBallotTab.CANDIDATES)}
                        aria-controls={panelId(ESlateBallotTab.CANDIDATES)}
                        sx={TAB_STYLE}
                    />
                </Tabs>
            </Box>
            {panels.map(([tab, content]) => (
                <div
                    key={tab}
                    className={`slate-ballot-panel slate-ballot-panel-${tab}`}
                    role="tabpanel"
                    hidden={value !== tab}
                    id={panelId(tab)}
                    aria-labelledby={tabId(tab)}
                >
                    {content}
                </div>
            ))}
        </Box>
    )
}
