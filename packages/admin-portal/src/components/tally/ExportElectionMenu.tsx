import {Box, Menu, MenuItem} from "@mui/material"
import Button from "@mui/material/Button"
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {EXPORT_FORMATS} from "./constants"
import {
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Session,
} from "@/gql/graphql"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"

interface ExportElectionMenuProps {
    resource: string
    event?: Sequent_Backend_Tally_Session
    election?: Sequent_Backend_Election
    contest?: Sequent_Backend_Contest
    area?: Sequent_Backend_Area_Contest | string | undefined
}

const ExportButton = styled.div`
    cursor: pointer;
    margin-left: 10px;
    margin-right: 10px;
    padding: 5px 10px;
    background-color: transparent;
    color: ${theme.palette.primary.dark};
    font-size: 14px;
    font-weight: 500;
    line-height: 1.5;
    text-align: center;
    text-transform: uppercase;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    &:hover {
        background-color: ${theme.palette.primary.dark};
        color: ${theme.palette.white};
    }
`

export const ExportElectionMenu: React.FC<ExportElectionMenuProps> = (props) => {
    const {resource, event, election, contest, area} = props
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)

    const handleMenu = (event: React.MouseEvent<HTMLElement>) => {
        event.preventDefault()
        event.stopPropagation()
        setAnchorEl(event.currentTarget)
    }

    const handleClose = () => {
        setAnchorEl(null)
    }

    const handleExport = (type: string) => {
        console.log("ExportElectionData :: ", resource)
        console.log("======================")

        console.log("ExportElectionData :: ", event)
        console.log("ExportElectionData :: ", election)
        console.log("ExportElectionData :: ", contest)
        console.log("ExportElectionData :: ", area)
        console.log("ExportElectionData :: ", type)
    }

    return (
        <div>
            <ExportButton
                aria-label="export election data"
                aria-controls="export-menu"
                aria-haspopup="true"
                onClick={handleMenu}
            >
                <span title={"common.label.export"}>{t("common.label.export")}</span>
            </ExportButton>

            <Menu
                id="menu-appbar"
                anchorEl={anchorEl}
                anchorOrigin={{
                    vertical: "bottom",
                    horizontal: "right",
                }}
                keepMounted
                transformOrigin={{
                    vertical: "top",
                    horizontal: "right",
                }}
                sx={{maxWidth: 220}}
                open={Boolean(anchorEl)}
                onClose={handleClose}
            >
                {EXPORT_FORMATS.map((format: {label: string; value: string}) => (
                    <MenuItem
                        key={format.value}
                        onClick={(e: React.MouseEvent<HTMLElement>) => {
                            e.preventDefault()
                            e.stopPropagation()
                            handleClose()
                            handleExport(format.value)
                        }}
                    >
                        <Box
                            sx={{
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                            }}
                        >
                            <span title={format.label}>{format.label}</span>
                        </Box>
                    </MenuItem>
                ))}
            </Menu>
        </div>
    )
}
