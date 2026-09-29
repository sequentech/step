// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {IconButton, Menu, MenuItem} from "@mui/material"
import MoreHorizIcon from "@mui/icons-material/MoreHoriz"
import {useTranslation} from "react-i18next"

export interface MonitoringWidgetMenuProps {
    widgetTitle: string
    /** Each action is offered only when given; View data and Export need figures. */
    onConfigure?: () => void
    onViewData?: () => void
    onExport?: () => void
    onDuplicate?: () => void
}

/** The ⋯ menu: Configure widget · View data · Export CSV · Duplicate. */
export function MonitoringWidgetMenu({
    widgetTitle,
    onConfigure,
    onViewData,
    onExport,
    onDuplicate,
}: MonitoringWidgetMenuProps) {
    const {t} = useTranslation()
    const [anchor, setAnchor] = useState<HTMLElement | null>(null)
    const choose = (action?: () => void) => () => {
        setAnchor(null)
        action?.()
    }
    const items = [
        {
            key: "configure",
            label: t("monitoring.widget.configure"),
            action: onConfigure,
            always: false,
        },
        {key: "viewData", label: t("monitoring.widget.viewData"), action: onViewData, always: true},
        {key: "export", label: t("monitoring.widget.exportCsv"), action: onExport, always: true},
        {
            key: "duplicate",
            label: t("monitoring.widget.duplicate"),
            action: onDuplicate,
            always: false,
        },
    ].filter((item) => item.always || item.action)
    return (
        <>
            <IconButton
                size="small"
                aria-label={t("monitoring.widget.menu", {widget: widgetTitle})}
                aria-haspopup="menu"
                onClick={(event) => setAnchor(event.currentTarget)}
            >
                <MoreHorizIcon fontSize="small" />
            </IconButton>
            <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
                {items.map((item) => (
                    <MenuItem key={item.key} disabled={!item.action} onClick={choose(item.action)}>
                        {item.label}
                    </MenuItem>
                ))}
            </Menu>
        </>
    )
}
