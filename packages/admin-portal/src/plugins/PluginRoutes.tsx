// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {Suspense, useMemo, useRef} from "react"
import {useParams} from "react-router-dom"
import {Alert, Box, Button, CircularProgress} from "@mui/material"
import {Menu, useSidebarState} from "react-admin"
import TimelineIcon from "@mui/icons-material/Timeline"
import {useTranslation} from "react-i18next"
import {usePlugins} from "./PluginProvider"
import {contributions} from "./registry"
import {Compatibility, PluginContribution, PluginHost, PluginState, TenantPlugin} from "./contracts"

const usable = (
    plugin: TenantPlugin | undefined,
    item: PluginContribution
): plugin is TenantPlugin =>
    Boolean(
        plugin &&
        plugin.state === PluginState.Enabled &&
        plugin.compatibility === Compatibility.Compatible &&
        plugin.manifest.api_major === item.apiMajor
    )

/** Menu items for enabled plugins the user may read, after Templates. */
export const PluginMenu: React.FC = () => {
    const {plugins, can} = usePlugins()
    const [open] = useSidebarState()
    const {t} = useTranslation()
    return (
        <>
            {contributions
                .filter((item) => {
                    const plugin = plugins.find((candidate) => candidate.manifest.id === item.id)
                    return usable(plugin, item) && can(plugin.manifest.read_permission)
                })
                .map((item) => (
                    <Menu.Item
                        key={item.id}
                        to={item.route}
                        primaryText={open ? t(item.labelKey) : ""}
                        leftIcon={<TimelineIcon />}
                    />
                ))}
        </>
    )
}

/** Renders a plugin's screen, or explains why it is unavailable. */
export const PluginRoute: React.FC = () => {
    const {id} = useParams()
    const context = usePlugins()
    const {t} = useTranslation()
    const latest = useRef(context)
    latest.current = context
    // A stable host keeps the plugin's state across the portal's re-renders.
    const host = useMemo<PluginHost>(
        () => ({
            can: (permission) => latest.current.can(permission),
            execute: (operation, data) => latest.current.execute(id ?? "", operation, data),
        }),
        [id]
    )
    if (context.loading && !context.plugins.length) {
        return <CircularProgress aria-label={t("plugins.loading")} />
    }
    if (context.error) {
        return (
            <Alert
                severity="error"
                action={
                    <Button onClick={() => void context.refresh()}>{t("plugins.retry")}</Button>
                }
            >
                {context.error}
            </Alert>
        )
    }
    const item = contributions.find((candidate) => candidate.id === id)
    const plugin = context.plugins.find((candidate) => candidate.manifest.id === id)
    if (!item || !plugin) return <Alert severity="warning">{t("plugins.unavailable")}</Alert>
    const name = t(item.labelKey)
    if (
        plugin.compatibility !== Compatibility.Compatible ||
        plugin.manifest.api_major !== item.apiMajor
    ) {
        return <Alert severity="warning">{t("plugins.incompatibleRoute")}</Alert>
    }
    if (!usable(plugin, item))
        return <Alert severity="info">{t("plugins.disabledRoute", {name})}</Alert>
    if (!context.can(plugin.manifest.read_permission)) {
        return <Alert severity="warning">{t("plugins.forbidden", {name})}</Alert>
    }
    const Component = item.component
    return (
        <Box sx={{p: {xs: 2, md: 3}, minWidth: 0, width: "100%", boxSizing: "border-box"}}>
            <Suspense fallback={<CircularProgress aria-label={t("plugins.loading")} />}>
                <Component key={id} host={host} />
            </Suspense>
        </Box>
    )
}
