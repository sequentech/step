// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {Drawer} from "@mui/material"
import {Add} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {ImportConfig} from "react-admin-import-csv"
import DownloadIcon from "@mui/icons-material/Download"
import UploadIcon from "@mui/icons-material/Upload"

import {Button, TopToolbar, FilterButton, SelectColumnsButton, ExportButton} from "react-admin"
import {preferencePanelStateInitializer} from "@mui/x-data-grid/internals"

interface ListActionsProps {
    withColumns?: boolean
    withImport?: boolean
    doImport?: () => void
    withExport?: boolean
    doExport?: () => void
    openExportMenu?: (e: React.MouseEvent<HTMLElement>) => void
    isExportDisabled?: boolean
    withFilter?: boolean
    withAction?: boolean
    open?: boolean
    setOpen?: (val: boolean) => void
    doAction?: () => void
    actionLabel?: string
    Component?: React.ReactNode
    withComponent?: boolean
    custom?: boolean
    extraActions?: Array<any>
    defaultExport?: boolean
    preferenceKey?: string
}

export const ListActions: React.FC<ListActionsProps> = (props) => {
    const {
        withColumns = true,
        preferenceKey,
        withImport = true,
        doImport = () => {},
        withExport = true,
        doExport = () => {},
        openExportMenu = () => {},
        isExportDisabled = false,
        withFilter = true,
        withAction = false,
        doAction = () => {},
        actionLabel = "",
        Component,
        withComponent,
        open = false,
        setOpen = () => {},
        custom = true,
        extraActions = [],
        defaultExport = false,
    } = props

    const exportWithOptions = props.openExportMenu !== undefined

    const {t} = useTranslation()

    const config: ImportConfig = {
        logging: true,
        disableCreateMany: true,
        disableUpdateMany: true,
    }

    return (
        <div className={custom ? "list-actions" : ""}>
            <TopToolbar
                sx={{
                    backgroundColor: "transparent",
                    display: "flex",
                }}
            >
                {withColumns ? (
                    preferenceKey ? (
                        <SelectColumnsButton preferenceKey={preferenceKey} />
                    ) : (
                        <SelectColumnsButton />
                    )
                ) : null}

                {withFilter ? <FilterButton /> : null}

                {withAction ? (
                    <Button onClick={doAction} label={String(t(actionLabel))}>
                        <Add />
                    </Button>
                ) : null}

                {withComponent && Component && (
                    <>
                        <Button
                            onClick={() => setOpen(true)}
                            label={String(t("common.label.add"))}
                            className="add-button"
                        >
                            <Add />
                        </Button>

                        <Drawer
                            anchor="right"
                            open={open}
                            onClose={() => {
                                setOpen(false)
                            }}
                            PaperProps={{
                                sx: {width: "30%"},
                            }}
                        >
                            {Component}
                        </Drawer>
                    </>
                )}

                {withImport ? (
                    <Button onClick={doImport} label={String(t("common.label.import"))}>
                        <UploadIcon />
                    </Button>
                ) : null}

                {withExport && exportWithOptions ? (
                    <React.Fragment>
                        <Button
                            onClick={(e: React.MouseEvent<HTMLElement>) => openExportMenu(e)}
                            label={String(t("common.label.export"))}
                            disabled={isExportDisabled}
                        >
                            <DownloadIcon />
                        </Button>
                    </React.Fragment>
                ) : null}

                {withExport && !exportWithOptions ? (
                    !defaultExport ? (
                        <Button
                            onClick={doExport}
                            label={String(t("common.label.export"))}
                            disabled={isExportDisabled}
                        >
                            <DownloadIcon />
                        </Button>
                    ) : (
                        <ExportButton />
                    )
                ) : null}

                {extraActions.length > 0 ? extraActions : null}
            </TopToolbar>
        </div>
    )
}
