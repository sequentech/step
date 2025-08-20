// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {tallyQueryData} from "@/atoms/tally-candidates"
import {
    GetTallyDataQuery,
    Sequent_Backend_Area,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Results_Area_Contest,
    Sequent_Backend_Results_Area_Contest_Candidate,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
    Sequent_Backend_Results_Election,
    Sequent_Backend_Results_Election_Area,
    Sequent_Backend_Results_Event,
} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useSetAtom} from "jotai"
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useManagedDatabase, useSQLQuery} from "@/hooks/useSQLiteDatabase"
import {isString} from "@sequentech/ui-core"
import {IResultDocuments} from "@/types/results"

export interface ResultsDataLoaderProps {
    resultsEventId: string
    electionEventId: string
    isTallyCompleted: boolean
    contests: Array<Sequent_Backend_Contest>
    electionIds: Array<string>
    databaseName: string
}

export const ResultsDataLoader: React.FC<ResultsDataLoaderProps> = ({
    resultsEventId,
    electionEventId,
    isTallyCompleted,
    contests,
    electionIds,
    databaseName,
}) => {
    const [tenantId] = useTenantStore()
    const setTallyQueryData = useSetAtom(tallyQueryData)

    const contestIds = useMemo(() => contests.map((c) => c.id), [contests])

    const {isLoading: isDbLoading, error: dbError} = useManagedDatabase(
        databaseName,
        electionEventId ? electionEventId : undefined
    )

    const {data: area} = useSQLQuery(
        `SELECT * FROM area WHERE election_event_id = ? and tenant_id = ?`,
        [electionEventId, tenantId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!electionEventId && !!tenantId,
        }
    )

    const {data: area_contest} = useSQLQuery(
        `SELECT * FROM area_contest WHERE election_event_id = ? and tenant_id = ? and contest_id in (${contestIds
            .map(() => "?")
            .join(",")})`,
        [electionEventId, tenantId, ...contestIds],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!electionEventId && !!tenantId && contestIds.length > 0,
        }
    )

    const {data: election} = useSQLQuery(
        `SELECT * FROM election WHERE election_event_id = ? and tenant_id = ? and id in (${electionIds
            .map(() => "?")
            .join(",")})`,
        [electionEventId, tenantId, ...electionIds],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!electionEventId && !!tenantId && electionIds.length > 0,
        }
    )

    const {data: candidate} = useSQLQuery(
        `SELECT * FROM candidate WHERE election_event_id = ? and tenant_id = ? and contest_id in (${contestIds
            .map(() => "?")
            .join(",")})`,
        [electionEventId, tenantId, ...contestIds],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!electionEventId && !!tenantId && contestIds.length > 0,
        }
    )

    const {data: contest} = useSQLQuery(
        `SELECT * FROM contest WHERE election_event_id = ? and tenant_id = ? and id in (${contestIds
            .map(() => "?")
            .join(",")})`,
        [electionEventId, tenantId, ...contestIds],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!electionEventId && !!tenantId && contestIds.length > 0,
        }
    )

    const {data: results_event} = useSQLQuery(
        `SELECT * FROM results_event WHERE election_event_id = ? and tenant_id = ? and id = ?`,
        [electionEventId, tenantId, resultsEventId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!resultsEventId && !!electionEventId && !!tenantId,
        }
    )

    const {data: results_election} = useSQLQuery(
        `SELECT * FROM results_election WHERE election_event_id = ? and tenant_id = ? and results_event_id = ?`,
        [electionEventId, tenantId, resultsEventId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!resultsEventId && !!electionEventId && !!tenantId,
        }
    )

    const {data: results_contest_candidate} = useSQLQuery(
        `SELECT * FROM results_contest_candidate WHERE election_event_id = ? and tenant_id = ? and results_event_id = ?`,
        [electionEventId, tenantId, resultsEventId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!resultsEventId && !!electionEventId && !!tenantId,
        }
    )

    const {data: results_contest} = useSQLQuery(
        `SELECT * FROM results_contest WHERE election_event_id = ? and tenant_id = ? and results_event_id = ?`,
        [electionEventId, tenantId, resultsEventId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!resultsEventId && !!electionEventId && !!tenantId,
        }
    )

    const {data: results_area_contest_candidate} = useSQLQuery(
        `SELECT * FROM results_area_contest_candidate WHERE election_event_id = ? and tenant_id = ? and results_event_id = ?`,
        [electionEventId, tenantId, resultsEventId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!resultsEventId && !!electionEventId && !!tenantId,
        }
    )

    const {data: results_area_contest} = useSQLQuery(
        `SELECT * FROM results_area_contest WHERE election_event_id = ? and tenant_id = ? and results_event_id = ?`,
        [electionEventId, tenantId, resultsEventId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!resultsEventId && !!electionEventId && !!tenantId,
        }
    )

    const {data: results_election_area} = useSQLQuery(
        `SELECT * FROM results_election_area WHERE election_event_id = ? and tenant_id = ? and results_event_id = ?`,
        [electionEventId, tenantId, resultsEventId],
        {
            databaseName: databaseName,
            enabled: !isDbLoading && !!resultsEventId && !!electionEventId && !!tenantId,
        }
    )

    let tallyData: GetTallyDataQuery = {
        sequent_backend_area: area as Sequent_Backend_Area[],
        sequent_backend_area_contest: area_contest as Sequent_Backend_Area_Contest[],
        sequent_backend_election: election as Sequent_Backend_Election[],
        sequent_backend_candidate: candidate as Sequent_Backend_Candidate[],
        sequent_backend_contest: contest as Sequent_Backend_Contest[],
        sequent_backend_results_event: results_event as Sequent_Backend_Results_Event[],
        sequent_backend_results_election: results_election as Sequent_Backend_Results_Election[],
        sequent_backend_results_contest_candidate:
            results_contest_candidate as Sequent_Backend_Results_Contest_Candidate[],
        sequent_backend_results_contest: results_contest.map((contest) => {
            if (isString(contest.documents)) {
                try {
                    contest.documents = JSON.parse(contest.documents) as IResultDocuments
                } catch (e) {
                    console.error("error parsing contest documents" + e)
                }
            }
            return contest
        }) as Sequent_Backend_Results_Contest[],
        sequent_backend_results_area_contest_candidate:
            results_area_contest_candidate as Sequent_Backend_Results_Area_Contest_Candidate[],
        sequent_backend_results_area_contest: results_area_contest.map((contest) => {
            if (isString(contest.documents)) {
                try {
                    contest.documents = JSON.parse(contest.documents) as IResultDocuments
                } catch (e) {
                    console.error("error parsing contest documents" + e)
                }
            }
            return contest
        }) as Sequent_Backend_Results_Area_Contest[],
        sequent_backend_results_election_area:
            results_election_area as Sequent_Backend_Results_Election_Area[],
    }

    useEffect(() => {
        console.log("ResultsDataLoader tallyData: ", tallyData)
        setTallyQueryData(tallyData ?? null)
    }, [tallyData])

    return <></>
}
