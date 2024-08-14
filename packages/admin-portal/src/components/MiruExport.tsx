// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {Box, Menu, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useAtomValue} from "jotai"
import {
    Sequent_Backend_Results_Area_Contest,
    Sequent_Backend_Area,
    CreateTransmissionPackageMutation,
    Sequent_Backend_Tally_Session,
} from "@/gql/graphql"
import {uniq} from "lodash"
import {IPermissions} from "@/types/keycloak"
import {useMutation} from "@apollo/client"
import {CREATE_TRANSMISSION_PACKAGE} from "@/queries/CreateTransmissionPackage"
import {IMiruTallySessionData, MIRU_TALLY_SESSION_ANNOTATION_KEY} from "@/types/miru"
import {useNotify} from "react-admin"

export const ExportButton = styled.div`
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

interface MiruExportProps {
    electionId: string | null
    tally: Sequent_Backend_Tally_Session | undefined
	onSuccess?: () => void
}

export const MiruExport: React.FC<MiruExportProps> = ({electionId, tally, onSuccess}) => {
    const {t} = useTranslation()
    const tallyData = useAtomValue(tallyQueryData)
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const notify = useNotify()

    const [CreateTransmissionPackage] = useMutation<CreateTransmissionPackageMutation>(
        CREATE_TRANSMISSION_PACKAGE,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.TALLY_WRITE,
                },
            },
        }
    )

    const tallySessionData: IMiruTallySessionData = useMemo(() => {
        try {
            let strData = tally?.annotations?.[MIRU_TALLY_SESSION_ANNOTATION_KEY]
            if (!strData) {
                return []
            }
            return JSON.parse(strData) as IMiruTallySessionData
        } catch (e) {
            return []
        }
    }, [tally?.annotations?.[MIRU_TALLY_SESSION_ANNOTATION_KEY]])

    const resultsAreaContests: Array<Sequent_Backend_Results_Area_Contest> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_area_contest?.filter(
                (resultsContest) => electionId === resultsContest.election_id
            ),
        [tallyData?.sequent_backend_results_contest, electionId]
    )
    const areaIds: Array<string> = useMemo(() => {
        let areaIds = resultsAreaContests?.map((value) => value.area_id) ?? []
        return uniq(areaIds)
    }, [resultsAreaContests])

    const areas: Array<Sequent_Backend_Area> = useMemo(
        () => tallyData?.sequent_backend_area?.filter((area) => areaIds.includes(area.id)) ?? [],
        [areaIds, tallyData?.sequent_backend_area]
    )

    const handleMenu = (event: React.MouseEvent<HTMLElement>) => {
        event.preventDefault()
        event.stopPropagation()
        setAnchorEl(event.currentTarget)
    }

    const handleClose = () => {
        setAnchorEl(null)
    }

    const handleCreateTransmissionPackage = async (areaId: string) => {

				onSuccess?.()


		return
        const found = tallySessionData.find(
            (datum) => datum.areaId === areaId && datum.electionId === electionId
        )

        if (found) {
            notify("Already exists: transmission package", {type: "success"})
            return
        }

        try {
            const {data: nextStatus, errors} = await CreateTransmissionPackage({
                variables: {
                    electionId: electionId,
                    tallySessionId: tally?.id,
                    areaId,
                },
            })

            if (errors) {
                notify("Error creating transmission package", {type: "error"})
                return
            }

            if (nextStatus) {
                notify("Success creating transmission package", {type: "success"})
				onSuccess?.()
            }
        } catch (error) {
            notify("Error creating transmission package", {type: "error"})
        }
    }

    return (
        <Box>
            <ExportButton
                aria-label="export election data"
                aria-controls="export-menu"
                aria-haspopup="true"
                onClick={handleMenu}
            >
                <span title={t("common.label.export")}>{t("common.label.export")}</span>
            </ExportButton>

            <Menu
                id="menu-export-election"
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
                sx={{maxWidth: 620}}
                open={Boolean(anchorEl)}
                onClose={handleClose}
            >
                {areas.map((area) => (
                    <MenuItem
                        key={area.id}
                        onClick={(e: React.MouseEvent<HTMLElement>) => {
                            e.preventDefault()
                            e.stopPropagation()
                            handleClose()
                            handleCreateTransmissionPackage(area.id)
                            //handleExport(format.value)
                        }}
                    >
                        <Box
                            sx={{
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                            }}
                        >
                            <span>
                                {t("tally.exportElectionArea", {
                                    name: area.name,
                                })}
                            </span>
                        </Box>
                    </MenuItem>
                ))}
            </Menu>
        </Box>
    )
}
