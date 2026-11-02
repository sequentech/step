// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {
    Alert,
    Box,
    Button,
    Card,
    CardContent,
    Chip,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Stack,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {usePlugins} from "./PluginProvider"
import {Compatibility, PluginState, TenantPlugin, errorMessage} from "./contracts"
export const PluginSettings = () => {
    const context = usePlugins()
    const {t} = useTranslation()
    const [selected, setSelected] = useState<TenantPlugin | null>(null)
    const [busy, setBusy] = useState(false)
    const [error, setError] = useState("")
    const change = async () => {
        if (!selected) return
        setBusy(true)
        setError("")
        try {
            await context.setState(
                selected,
                selected.state === PluginState.Enabled ? PluginState.Disabled : PluginState.Enabled
            )
            setSelected(null)
        } catch (cause) {
            setError(errorMessage(cause))
        } finally {
            setBusy(false)
        }
    }
    return (
        <Box sx={{p: 2}}>
            <Typography variant="h5">{t("plugins.title")}</Typography>
            <Typography sx={{my: 2}}>{t("plugins.description")}</Typography>
            {(error || context.error) && (
                <Alert
                    severity="error"
                    action={
                        <Button onClick={() => void context.refresh()}>{t("plugins.retry")}</Button>
                    }
                >
                    {error || context.error}
                </Alert>
            )}
            {!context.can("tenant-write") && (
                <Alert severity="info" sx={{mb: 2}}>
                    {t("plugins.noPermission")}
                </Alert>
            )}
            <Stack spacing={2}>
                {context.plugins.map((plugin) => (
                    <Card variant="outlined" key={plugin.manifest.id}>
                        <CardContent>
                            <Stack direction="row" spacing={2} alignItems="center">
                                <Typography variant="h6">{t(plugin.manifest.label_key)}</Typography>
                                <Chip label={plugin.manifest.version} />
                                <Chip label={t(`plugins.${plugin.state}`)} />
                            </Stack>
                            <Typography sx={{my: 1}}>
                                {t("plugins.api", {major: plugin.manifest.api_major})} ·{" "}
                                {t(`plugins.${plugin.compatibility}`)}
                            </Typography>
                            <Typography>
                                {t("plugins.permissions")}: {plugin.manifest.permissions.join(", ")}{" "}
                                · {t("plugins.adminGroup")}
                            </Typography>
                            <Button
                                sx={{mt: 2}}
                                disabled={
                                    !context.can("tenant-write") ||
                                    (plugin.compatibility !== Compatibility.Compatible &&
                                        plugin.state !== PluginState.Enabled)
                                }
                                onClick={() => {
                                    setSelected(plugin)
                                    setError("")
                                }}
                            >
                                {t(
                                    plugin.state === PluginState.Enabled
                                        ? "plugins.disable"
                                        : "plugins.enable"
                                )}
                            </Button>
                        </CardContent>
                    </Card>
                ))}
            </Stack>
            <Dialog open={Boolean(selected)} onClose={() => !busy && setSelected(null)}>
                <DialogTitle>
                    {t(
                        selected?.state === PluginState.Enabled
                            ? "plugins.confirmDisableTitle"
                            : "plugins.confirmEnableTitle",
                        {name: selected ? t(selected.manifest.label_key) : ""}
                    )}
                </DialogTitle>
                <DialogContent>
                    {t(
                        selected?.state === PluginState.Enabled
                            ? "plugins.confirmDisable"
                            : "plugins.confirmEnable",
                        {name: selected ? t(selected.manifest.label_key) : ""}
                    )}
                    {error && <Alert severity="error">{error}</Alert>}
                </DialogContent>
                <DialogActions>
                    <Button disabled={busy} onClick={() => setSelected(null)}>
                        {t("projects.cancel")}
                    </Button>
                    <Button disabled={busy} onClick={() => void change()}>
                        {t(busy ? "projects.editor.saving" : "plugins.confirm")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Box>
    )
}
