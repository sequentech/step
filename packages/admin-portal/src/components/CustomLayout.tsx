// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// import React from "react"
import React, {useContext, useEffect} from "react"
import {Layout, LayoutProps, SidebarClasses} from "react-admin"
import {CustomAppBar} from "./CustomAppBar"
import {CustomMenu} from "./CustomMenu"
import {CustomSidebar} from "./menu/CustomSidebar"
import {TenantContext} from "@/providers/TenantContextProvider"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import { useGetOne } from 'react-admin';
import {styled} from "@mui/material/styles"

const SequentSidebar = (props: any) => {
    return (
        <CustomSidebar {...props}>
            <CustomMenu {...props} classes={SidebarClasses} />
        </CustomSidebar>
    )
}

const StyledLayout = styled(Layout)<{ css: string }>`
    ${({ css }) => css}
    & .MuiPaper-root.RaSidebar-paper, & .MuiPaper-root.MuiAppBar-root {
        top: 0;
        position: sticky;
        zIndex: 100;
    }
    & .MuiToolbar-root {
        minHeight: unset;
    }
    & .RaList-main {
        width: 50%;
    }
`;

export const CustomLayout: React.FC<LayoutProps> = (props) => {

    const {tenantId} = useContext(TenantContext)
    const {data: tenantData} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
      id: tenantId,
    })
    const customCss = tenantData?.annotations?.css ?? ""
    console.log("CustomLayout: customCss :: ", customCss)

    return (
        <StyledLayout
            {...props}
            css={customCss}
            appBar={CustomAppBar}
            sidebar={SequentSidebar}
        />
    )
}