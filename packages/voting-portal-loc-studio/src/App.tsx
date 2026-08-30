// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef} from "react"
import {Box, Button, MenuItem, Select, Typography} from "@mui/material"
import {useCurrentScene, useLocStudio} from "./LocStudioContext"
import {SceneNav} from "./SceneNav"
import {PreviewFrame} from "./PreviewFrame"
import {KeyPanel} from "./KeyPanel"
import {exportUploadedElectionEvent} from "./uploadedElection"
import {ElectionImportDialog} from "./ElectionImportDialog"
import {buildLocStudioSaveResult, LocStudioSaveResult} from "./saveResult"

const LANGUAGE_LABELS: Record<string, string> = {
    en: "English",
    es: "Spanish",
    cat: "Catalan",
    fr: "French",
    tl: "Tagalog",
    gl: "Galician",
    nl: "Dutch",
    eu: "Basque",
}

const downloadFile = (fileName: string, content: string): void => {
    const blob = new Blob([content], {type: "application/json"})
    const url = URL.createObjectURL(blob)
    const link = document.createElement("a")
    link.href = url
    link.download = fileName
    link.click()
    URL.revokeObjectURL(url)
}

export const LocStudioApp: React.FC<{
    embedded?: boolean
    onSave?: (result: LocStudioSaveResult) => void | Promise<void>
    saving?: boolean
}> = ({embedded = false, onSave, saving = false}) => {
    const {
        language,
        languageOptions,
        setLanguage,
        overrides,
        importOverrides,
        resetAllOverrides,
        uploadedEvent,
        clearUploadedEvent,
        openImportDialog,
    } = useLocStudio()
    const {scene, variant} = useCurrentScene()
    const fileInputRef = useRef<HTMLInputElement>(null)

    const exportOverrides = () => {
        downloadFile("presentation-i18n.json", JSON.stringify(overrides, null, 2))
    }

    const onImportUiStrings = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0]
        if (!file) {
            return
        }
        try {
            const parsed = JSON.parse(await file.text()) as Record<string, Record<string, string>>
            importOverrides(parsed)
        } catch {
            window.alert("Could not import that JSON file. Use presentation.i18n format.")
        }
        event.target.value = ""
    }

    const exportElectionEvent = () => {
        if (!uploadedEvent) {
            return
        }
        const {fileName, content} = exportUploadedElectionEvent(uploadedEvent, overrides)
        downloadFile(fileName, content)
    }

    const handleSave = () => {
        if (!onSave) {
            return
        }
        void onSave(buildLocStudioSaveResult(uploadedEvent, overrides))
    }

    return (
        <Box className={embedded ? "loc-studio-root loc-studio-embedded" : "loc-studio-root"}>
            <Box className="loc-studio-toolbar">
                <Box>
                    <Typography className="loc-studio-brand">
                        {embedded ? "Localization Studio" : "Voting Portal Loc Studio"}
                    </Typography>
                    <Typography className="loc-studio-subtitle">
                        {scene.label} / {variant.label}
                        {uploadedEvent ? ` · ${uploadedEvent.fileName}` : ""}
                    </Typography>
                </Box>
                <Box className="loc-studio-toolbar-actions">
                    <Select
                        size="small"
                        value={language}
                        onChange={(event) => setLanguage(event.target.value)}
                    >
                        {languageOptions.map((code) => (
                            <MenuItem key={code} value={code}>
                                {LANGUAGE_LABELS[code] || code}
                            </MenuItem>
                        ))}
                    </Select>
                    {embedded && onSave ? (
                        <Button size="small" variant="primary" disabled={saving} onClick={handleSave}>
                            {saving ? "Saving…" : "Save"}
                        </Button>
                    ) : null}
                    {!embedded ? (
                        <Button size="small" variant="secondary" onClick={openImportDialog}>
                            Import election event
                        </Button>
                    ) : null}
                    {!embedded ? (
                        <Button
                            size="small"
                            variant="secondary"
                            disabled={!uploadedEvent}
                            onClick={exportElectionEvent}
                        >
                            Export election event
                        </Button>
                    ) : null}
                    {!embedded && uploadedEvent ? (
                        <Button size="small" variant="secondary" onClick={clearUploadedEvent}>
                            Use sample data
                        </Button>
                    ) : null}
                    {!embedded ? (
                        <Button size="small" variant="secondary" onClick={exportOverrides}>
                            Export UI strings
                        </Button>
                    ) : null}
                    {!embedded ? (
                        <Button
                            size="small"
                            variant="secondary"
                            onClick={() => fileInputRef.current?.click()}
                        >
                            Import UI strings
                        </Button>
                    ) : null}
                    <Button size="small" variant="secondary" onClick={resetAllOverrides}>
                        Reset all
                    </Button>
                    <input
                        ref={fileInputRef}
                        type="file"
                        accept="application/json"
                        hidden
                        onChange={onImportUiStrings}
                    />
                </Box>
            </Box>
            <Box className="loc-studio-body">
                <SceneNav />
                <PreviewFrame />
                <KeyPanel />
            </Box>
            <ElectionImportDialog />
        </Box>
    )
}
