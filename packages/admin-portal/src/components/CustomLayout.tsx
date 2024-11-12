// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
import {ITenantTheme} from "@sequentech/ui-core"
import {ImportDataDrawer} from "./election-event/import-data/ImportDataDrawer"
import {
    useCreateElectionEventStore,
    withCreateElectionEventProvider,
} from "@/providers/CreateElectionEventContextProvider"
import {CreateDataDrawer} from "./election-event/create/CreateElectionEventDrawer"

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

    return <></>
}

const SequentSidebar = (props: any) => {
    const {
        importDrawer,
        uploadCallback,
        handleImportElectionEvent,
        closeImportDrawer,
        errors,
        createDrawer,
        closeCreateDrawer,
        toggleImportDrawer,
        postDefaultValues,
        handleElectionCreated,
        handleSubmit,
    } = useCreateElectionEventStore()

    useEffect(() => {
        console.log("sidebar")
    }, [])

    return (
        <>
            <CustomCssReader />
            <CustomSidebar {...props}>
                <CustomMenu {...props} classes={SidebarClasses} />
            </CustomSidebar>
            <CreateDataDrawer open={createDrawer} closeDrawer={() => closeCreateDrawer?.()} />
            <ImportDataDrawer
                open={importDrawer}
                title="electionEventScreen.import.eetitle"
                subtitle="electionEventScreen.import.eesubtitle"
                paragraph={"electionEventScreen.import.electionEventParagraph"}
                closeDrawer={closeImportDrawer}
                doImport={handleImportElectionEvent}
                disableImport={!!errors}
                uploadCallback={uploadCallback}
                errors={errors}
            />
        </>
    )
}

export const CustomLayout: React.FC<LayoutProps> = (props) => (
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
        sidebar={withCreateElectionEventProvider(SequentSidebar)}
    />
)
