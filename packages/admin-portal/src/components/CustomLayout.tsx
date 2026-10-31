// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect} from "react"
import {Layout, LayoutProps, SidebarClasses} from "react-admin"
import {CustomAppBar} from "./CustomAppBar"
import {CustomMenu} from "./CustomMenu"
import {CustomSidebar} from "./menu/CustomSidebar"
import {TenantContext} from "@/providers/TenantContextProvider"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import {useGetOne} from "react-admin"
import cssInputLookAndFeel from "@/atoms/css-input-look-and-feel"
import {useAtomValue, useSetAtom} from "jotai"
import {applyConfigurationLanguagePolicy, ITenantSettings, ITenantTheme} from "@sequentech/ui-core"
import {ImportDataDrawer} from "./election-event/import-data/ImportDataDrawer"
import {CreateElectionEventProvider} from "@/providers/CreateElectionEventContextProvider"
import {CreateDataDrawer} from "./election-event/create/CreateElectionEventDrawer"

import {SigningProvider} from "./signing/SigningProvider"
import {SigningAction} from "@/lib/signing/types"
import type {ISigningPanelData} from "@/lib/signing/api"
import {ClosedVotingCard} from "@/resources/Publish/ClosedVotingCard"
import {
    ReportCompletionActions,
    TransmissionCompletionActions,
} from "@/resources/Reports/ReportSigning"

/** What a completed request of each protected action shows in its panel. */
const signingCompletionActions = {
    [SigningAction.TransmitResults]: (data: ISigningPanelData) => (
        <TransmissionCompletionActions data={data} />
    ),
    [SigningAction.CloseVoting]: (data: ISigningPanelData) => <ClosedVotingCard data={data} />,
    [SigningAction.GenerateElectionReturns]: (data: ISigningPanelData) => (
        <ReportCompletionActions data={data} />
    ),
    [SigningAction.GenerateReports]: (data: ISigningPanelData) => (
        <ReportCompletionActions data={data} />
    ),
}

export const CustomCssReader: React.FC = () => {
    const {tenantId} = useContext(TenantContext)
    const {data: tenantData} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: tenantId,
    })

    const setAtomValue = useSetAtom(cssInputLookAndFeel)
    const css = useAtomValue(cssInputLookAndFeel)
    useEffect(() => {
        const customCss = (tenantData?.annotations as ITenantTheme | undefined)?.css ?? ""
        if (css !== customCss) {
            setAtomValue(customCss)
        }
    }, [tenantData?.annotations?.css, setAtomValue, css])

    useEffect(() => {
        const settings = tenantData?.settings
        if (settings) {
            applyConfigurationLanguagePolicy(settings as ITenantSettings)
        }
    }, [tenantData?.settings?.language_conf])

    return <></>
}

const SequentSidebar = (props: any) => {
    return (
        <CreateElectionEventProvider>
            <CustomCssReader />
            <CustomSidebar {...props}>
                <CustomMenu {...props} classes={SidebarClasses} />
            </CustomSidebar>
            <CreateDataDrawer />
            <ImportDataDrawer
                title="electionEventScreen.import.eetitle"
                subtitle="electionEventScreen.import.eesubtitle"
                paragraph={"electionEventScreen.import.electionEventParagraph"}
            />
        </CreateElectionEventProvider>
    )
}

export const CustomLayout: React.FC<LayoutProps> = (props) => (
    <SigningProvider completionActions={signingCompletionActions}>
        <Layout
            {...props}
            sx={{
                "width": "100%",
                "& .MuiPaper-root.RaSidebar-paper, & .MuiPaper-root.MuiAppBar-root": {
                    top: "0",
                    position: "sticky",
                    zIndex: 100,
                },
                "& .MuiToolbar-root": {
                    minHeight: "unset",
                },
                "& .RaList-main": {
                    width: "50%",
                },
            }}
            appBar={CustomAppBar}
            // sidebar={withCreateElectionEventProvider(SequentSidebar)}
            sidebar={SequentSidebar}
        />
    </SigningProvider>
)
