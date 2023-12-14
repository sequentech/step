import React, {useEffect, useState} from "react"

import {Drawer} from "@mui/material"
import {Add} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {ImportButton, ImportConfig} from "react-admin-import-csv"

import {Button, TopToolbar, ExportButton, FilterButton, SelectColumnsButton} from "react-admin"

interface ListActionsProps {
    withColumns?: boolean
    withImport?: boolean
    withExport?: boolean
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
        withExport = true,
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
                        <Button onClick={() => setOpen(true)} label={t("common.label.add")}>
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
                    <ImportButton
                        sx={{
                            color: "#0F054C",
                            textAlign: "center",
                            fontSize: "14px",
                            fontStyle: "normal",
                            fontWeight: "500",
                            lineHeight: "normal",
                            letterSpacing: "normal",
                            textTransform: "uppercase",
                            border: "1px solid #0F054C",
                            borderRadius: "0px",
                            padding: "6px 12px",
                        }}
                        className="test-import-button"
                        {...props}
                        {...config}
                    />
                ) : null}

                {withExport ? <ExportButton /> : null}

                {extraActions.length > 0 && extraActions.map((item) => item)}
            </TopToolbar>
        </div>
    )
}
