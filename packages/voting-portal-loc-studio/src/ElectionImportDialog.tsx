// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useRef, useState} from "react"
import {Box, Button, Typography} from "@mui/material"
import {useLocStudio} from "./LocStudioContext"

const ACCEPT = "application/json,.json"

export const ElectionImportDialog: React.FC = () => {
    const {importDialogOpen, uploadError, loadUploadedEvent, closeImportDialog} = useLocStudio()
    const [dragging, setDragging] = useState(false)
    const [loading, setLoading] = useState(false)
    const inputRef = useRef<HTMLInputElement>(null)

    const handleFile = useCallback(
        async (file: File | undefined) => {
            if (!file || loading) {
                return
            }
            if (
                !file.name.toLowerCase().endsWith(".json") &&
                file.type &&
                file.type !== "application/json"
            ) {
                return
            }
            setLoading(true)
            try {
                await loadUploadedEvent(file)
            } finally {
                setLoading(false)
            }
        },
        [loadUploadedEvent, loading]
    )

    if (!importDialogOpen) {
        return null
    }

    const onDrop = (event: React.DragEvent) => {
        event.preventDefault()
        setDragging(false)
        void handleFile(event.dataTransfer.files?.[0])
    }

    return (
        <Box
            className="loc-studio-import-overlay"
            onClick={closeImportDialog}
            role="presentation"
        >
            <Box
                className={
                    dragging
                        ? "loc-studio-import-dialog dragging"
                        : loading
                          ? "loc-studio-import-dialog loading"
                          : "loc-studio-import-dialog"
                }
                onClick={(event) => event.stopPropagation()}
                onDragEnter={(event) => {
                    event.preventDefault()
                    setDragging(true)
                }}
                onDragOver={(event) => {
                    event.preventDefault()
                    setDragging(true)
                }}
                onDragLeave={(event) => {
                    event.preventDefault()
                    if (!event.currentTarget.contains(event.relatedTarget as Node)) {
                        setDragging(false)
                    }
                }}
                onDrop={onDrop}
                role="dialog"
                aria-modal="true"
                aria-labelledby="loc-studio-import-title"
            >
                <Typography id="loc-studio-import-title" className="loc-studio-import-title">
                    Import election event
                </Typography>
                <Box
                    className="loc-studio-upload-zone"
                    onClick={() => inputRef.current?.click()}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                            event.preventDefault()
                            inputRef.current?.click()
                        }
                    }}
                >
                    <Typography className="loc-studio-upload-title">
                        {loading ? "Loading election event…" : "Drop your JSON file here"}
                    </Typography>
                    <Typography className="loc-studio-upload-help">
                        Admin export with publications (<code>export_election_event-*.json</code>)
                        or a publications file with <code>ballot_styles</code>. You can also click
                        to browse.
                    </Typography>
                    {uploadError ? (
                        <Typography className="loc-studio-upload-error">{uploadError}</Typography>
                    ) : null}
                    <input
                        ref={inputRef}
                        type="file"
                        accept={ACCEPT}
                        hidden
                        onChange={(event) => {
                            void handleFile(event.target.files?.[0])
                            event.target.value = ""
                        }}
                    />
                </Box>
                <Box className="loc-studio-import-actions">
                    <Button size="small" variant="secondary" onClick={closeImportDialog}>
                        Cancel
                    </Button>
                </Box>
            </Box>
        </Box>
    )
}
