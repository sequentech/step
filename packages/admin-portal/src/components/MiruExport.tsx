// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {Box, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import {theme} from "@sequentech/ui-essentials"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useAtomValue} from "jotai"
import {Sequent_Backend_Results_Area_Contest, Sequent_Backend_Area} from "@/gql/graphql"
import {uniq} from "lodash"

export const ExportButton = styled("div")`
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
    handleClose: () => void
    electionId: string | null
    loading?: boolean
    onCreateTransmissionPackage: (v: {area_id: string; election_id: string}) => void
}

export const MiruExport: React.FC<MiruExportProps> = ({
    handleClose,
    electionId,
    loading,
    onCreateTransmissionPackage,
}) => {
    const {t} = useTranslation()
    const tallyData = useAtomValue(tallyQueryData)

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

    return (
        <Box>
            {areas.map((area) => (
                <MenuItem
                    key={area.id}
                    onClick={(e: React.MouseEvent<HTMLElement>) => {
                        e.preventDefault()
                        e.stopPropagation()
                        handleClose()
                        onCreateTransmissionPackage({
                            area_id: area.id,
                            election_id: electionId!,
                        })
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
        </Box>
    )
}
