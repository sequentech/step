// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {Sequent_Backend_Election, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {Box, Chip, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {
    IElectionEventPresentation,
    parseEntityPresentation,
    sortByPresentationOrder,
} from "@sequentech/ui-core"
import {orderItemsByIds} from "./utils"
import {EBallotBoxesReadiness, type IBallotBoxesSummary} from "@/services/tallyEligibility"
import {useZonedTime} from "@/hooks/useZonedTime"
import {outlinedWarningChipSx} from "@/components/dashboard/election/ballotBoxChip"

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    active: boolean
    /** Whether the row could be selected when `active` was last decided. */
    selectable: boolean
    name: string
}
interface TallyElectionsListProps {
    elections: Sequent_Backend_Election[] | undefined
    electionEventId: string
    disabled?: boolean
    update: (elections: Array<string>) => void
    keysCeremonyId: string | null
    tallySession?: Sequent_Backend_Tally_Session
    electionEventPresentation?: unknown
    /**
     * The ballot boxes of each election, when the event seals them at close
     * (VOTE-FREEZE). An election whose ballot boxes are not all sealed and on
     * the bulletin board cannot be selected.
     */
    ballotBoxes?: Record<string, IBallotBoxesSummary>
}

export const TallyElectionsList: React.FC<TallyElectionsListProps> = (props) => {
    const {
        disabled,
        elections,
        update,
        keysCeremonyId,
        tallySession: tallyData,
        electionEventPresentation,
        ballotBoxes,
    } = props

    const {t, i18n} = useTranslation()
    const aliasRenderer = useAliasRenderer()
    const zonedTime = useZonedTime()
    // A created tally keeps its elections; only a new one is limited by the seals.
    const isSelectable = (id: string) =>
        !!tallyData || !ballotBoxes || ballotBoxes[id]?.readiness === EBallotBoxesReadiness.READY

    const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election_Extended>>([])

    const filteredElections = useMemo(() => {
        if (!keysCeremonyId || tallyData) {
            return elections
        }
        return elections?.filter((election) => election.keys_ceremony_id === keysCeremonyId)
    }, [elections, keysCeremonyId, tallyData])

    const electionsOrder =
        parseEntityPresentation<IElectionEventPresentation>(
            electionEventPresentation
        )?.elections_order

    useEffect(() => {
        if (filteredElections) {
            const selectedElections = tallyData
                ? orderItemsByIds(filteredElections, tallyData.election_ids ?? [])
                : filteredElections
            setElectionsData((previousElections) => {
                // Polling refreshes the elections, so keep the admin's selection
                // and only select elections that were not listed before. A row
                // that could not be selected (its ballot boxes not ready, or
                // still loading) holds no choice: once it can be, it is selected.
                const previousSelection = new Map(
                    previousElections
                        .filter((election) => election.selectable)
                        .map((election) => [election.id, election.active])
                )
                const mappedElections: Array<Sequent_Backend_Election_Extended> =
                    selectedElections.map((election, index) => {
                        const id = election.id || ""
                        return {
                            ...election,
                            rowId: index,
                            id,
                            name: aliasRenderer(election.presentation),
                            active: isSelectable(id) && (previousSelection.get(id) ?? true),
                            selectable: isSelectable(id),
                        }
                    })
                return sortByPresentationOrder(mappedElections, electionsOrder, {
                    getLabel: (election) => election.name,
                    getPresentation: (election) => election.presentation,
                }).map((election, index) => ({...election, rowId: index}))
            })
        }
    }, [aliasRenderer, electionsOrder, filteredElections, tallyData, ballotBoxes])

    useEffect(() => {
        if (electionsData) {
            const temp: Array<string> = electionsData
                .filter((election) => election.active)
                .map((election) => election.id)
            update(temp)
        }
    }, [electionsData])

    const columns: GridColDef[] = [
        {
            field: `presentation.i18n[${i18n.language}].alias`,
            headerName: t("tally.table.elections"),
            flex: 1,
            minWidth: 140,
            editable: false,
            valueGetter(value, row) {
                return value ? value : aliasRenderer(row)
            },
        },
        ...(ballotBoxes
            ? [
                  {
                      field: "ballotBoxes",
                      headerName: t("tally.table.ballotBoxes"),
                      description: t("tally.ballotBoxes.help"),
                      flex: 1,
                      // The longest status ("Sealed, 1 of 2 on the bulletin board") stays
                      // whole; on a narrow screen the grid scrolls instead.
                      minWidth: 290,
                      editable: false,
                      sortable: false,
                      renderCell: (props: GridRenderCellParams<any>) =>
                          ballotBoxesChip(ballotBoxes[props.row.id]),
                  } satisfies GridColDef,
              ]
            : []),
        {
            field: "active",
            headerName: t("tally.table.selected"),
            editable: false,
            width: 100,
            renderCell: (props: GridRenderCellParams<any, boolean>) => (
                <Checkbox
                    checked={props.value}
                    inputProps={{"aria-label": aliasRenderer(props.row)}}
                    disabled={disabled || !isSelectable(props.row.id)}
                    onChange={() => handleConfirmChange(props.row)}
                />
            ),
        },
    ]

    function ballotBoxesChip(summary?: IBallotBoxesSummary) {
        if (!summary || summary.readiness === EBallotBoxesReadiness.LOADING) return null
        const {readiness, total, sealed, published, deadline} = summary
        const chip: {
            label: string
            color: "default" | "success" | "warning" | "error"
            variant: "filled" | "outlined"
        } =
            readiness === EBallotBoxesReadiness.READY
                ? {
                      label: t("tally.ballotBoxes.sealed", {sealed, total}),
                      color: "success",
                      variant: "filled",
                  }
                : readiness === EBallotBoxesReadiness.FAILED
                  ? {label: t("tally.ballotBoxes.failed"), color: "error", variant: "filled"}
                  : readiness === EBallotBoxesReadiness.PUBLISHING
                    ? {
                          label: t("tally.ballotBoxes.publishing", {published, total}),
                          color: "warning",
                          variant: "outlined",
                      }
                    : readiness === EBallotBoxesReadiness.SEALING
                      ? {
                            label: t("tally.ballotBoxes.sealing", {time: zonedTime(deadline)}),
                            color: "default",
                            variant: "outlined",
                        }
                      : readiness === EBallotBoxesReadiness.OVERDUE
                        ? {
                              label: t("tally.ballotBoxes.overdue"),
                              color: "warning",
                              variant: "outlined",
                          }
                        : readiness === EBallotBoxesReadiness.UNAVAILABLE
                          ? {
                                label: t("tally.ballotBoxes.unavailable"),
                                color: "warning",
                                variant: "outlined",
                            }
                          : {
                                label: t("tally.ballotBoxes.notSealed"),
                                color: "default",
                                variant: "outlined",
                            }
        return (
            <Chip
                size="small"
                {...chip}
                sx={
                    chip.color === "warning" && chip.variant === "outlined"
                        ? outlinedWarningChipSx
                        : undefined
                }
            />
        )
    }

    // Why each election that can't be selected can't, shown under the list
    // rather than only on hover (VOTE-FREEZE).
    const blocked =
        ballotBoxes && !tallyData
            ? electionsData.flatMap((election) => {
                  const readiness = ballotBoxes[election.id]?.readiness
                  return readiness &&
                      readiness !== EBallotBoxesReadiness.READY &&
                      readiness !== EBallotBoxesReadiness.LOADING
                      ? [
                            t("tally.ballotBoxes.blocked", {
                                name: aliasRenderer(election),
                                reason: t(`tally.ballotBoxes.reason.${readiness}`),
                            }),
                        ]
                      : []
              })
            : []

    function handleConfirmChange(clickedRow: any) {
        const updatedData: Array<Sequent_Backend_Election_Extended> = electionsData?.map((x) => {
            if (x.rowId === clickedRow.rowId) {
                return {
                    ...x,
                    active: !clickedRow.active,
                }
            }
            return x
        })
        setElectionsData(updatedData)
    }

    return (
        <>
            <DataGrid
                rows={electionsData}
                sx={{width: "100%"}}
                columns={columns}
                initialState={{
                    pagination: {
                        paginationModel: {
                            pageSize: 10,
                        },
                    },
                }}
                pageSizeOptions={[10, 20, 50, 100]}
                disableRowSelectionOnClick
            />
            {blocked.length ? (
                <Box
                    component="ul"
                    sx={{m: 0, mt: 1, pl: 2}}
                    className="tally-ballot-boxes-blocked"
                >
                    <Typography component="li" variant="body2" sx={{listStyle: "none", ml: -2}}>
                        {t("tally.ballotBoxes.help")}
                    </Typography>
                    {blocked.map((line) => (
                        <Typography component="li" variant="body2" key={line}>
                            {line}
                        </Typography>
                    ))}
                </Box>
            ) : null}
        </>
    )
}
