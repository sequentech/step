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

import {Button, TopToolbar, FilterButton, SelectColumnsButton} from "react-admin"

interface ListActionsProps {
    withColumns?: boolean
    withImport?: boolean
    doImport?: () => void
    withExport?: boolean
    doExport?: () => void
    withFilter?: boolean
    withAction?: boolean
    open?: boolean
    setOpen?: (val: boolean) => void
    doAction?: () => void
    actionLabel?: string
    Component?: React.ReactNode
    custom?: boolean
    extraActions?: Array<any>
}

export const ListActions: React.FC<ListActionsProps> = (props) => {
    const {
        withColumns = true,
        withImport = true,
        doImport = () => {},
        withExport = true,
        doExport = () => {},
        withFilter = true,
        withAction = false,
        doAction = () => {},
        actionLabel = "",
        Component,
        open = false,
        setOpen = () => {},
        custom = true,
        extraActions = [],
    } = props

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
                {withColumns ? <SelectColumnsButton /> : null}

                {withFilter ? <FilterButton /> : null}

                {withAction ? <Button onClick={doAction} label={t(actionLabel)} /> : null}

                {Component && (
                    <>
                        <Button
                            onClick={() => setOpen(true)}
                            label={t("common.label.add")}
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
                    <Button onClick={doImport} label={t("common.label.import")}>
                        <UploadIcon />
                    </Button>
                ) : // <ImportButton
                //     sx={{
                //         color: "#0F054C",
                //         textAlign: "center",
                //         fontSize: "14px",
                //         fontStyle: "normal",
                //         fontWeight: "500",
                //         lineHeight: "normal",
                //         letterSpacing: "normal",
                //         textTransform: "uppercase",
                //         border: "1px solid #0F054C",
                //         borderRadius: "0px",
                //         padding: "6px 12px",
                //     }}
                //     className="test-import-button"
                //     {...props}
                //     {...config}
                // />
                null}

                {withExport ? (
                    <Button onClick={doExport} label={t("common.label.export")}>
                        <DownloadIcon />
                    </Button>
                ) : // <ExportButton />
                null}

                {extraActions.length > 0 ? extraActions : null}
            </TopToolbar>
        </div>
    )
}
