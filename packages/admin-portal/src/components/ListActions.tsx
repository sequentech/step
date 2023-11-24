import React, {useEffect, useState} from "react"

import {Drawer} from "@mui/material"
import {Add} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {ImportButton, ImportConfig} from "react-admin-import-csv"

import {Button, TopToolbar, ExportButton, FilterButton, SelectColumnsButton} from "react-admin"

interface ListActionsProps {
    withImport?: boolean
    withExport?: boolean
    withFilter?: boolean
    closeDrawer?: string
    Component?: React.ReactNode
    custom?: boolean
}

export const ListActions: React.FC<ListActionsProps> = (props) => {
    const {
        withImport = true,
        withExport = true,
        withFilter = true,
        Component,
        closeDrawer = false,
        custom = false,
    } = props

    const {t} = useTranslation()
    const [open, setOpen] = useState<boolean>(false)

    useEffect(() => {
        setOpen(false)
    }, [closeDrawer])

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
                <SelectColumnsButton />

                {withFilter ? <FilterButton /> : null}

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
            </TopToolbar>
        </div>
    )
}
