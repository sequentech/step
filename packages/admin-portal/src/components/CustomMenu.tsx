// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useLocation} from "react-router-dom"
import React, {useEffect, useState} from "react"
import {Menu, useSidebarState, useGetList, useResourceContext} from "react-admin"
import {
    faThLarge,
    faUsers,
    faCog,
    faStar,
    faPlusCircle,
    faFileText,
    faAngleDoubleLeft,
    faAngleDoubleRight,
    faSearch,
} from "@fortawesome/free-solid-svg-icons"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {HorizontalBox} from "./HorizontalBox"
import {Box, MenuItem, Select, SelectChangeEvent, TextField} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link} from "react-router-dom"
import {TreeMenu} from "./TreeMenu"
import {useLocalStorage} from "react-use"

export function useTenantStore() {
    return useLocalStorage("tenantId")
}

const StyledItem = styled(Menu.Item)`
    color: ${adminTheme.palette.brandColor};

    &.RaMenuItemLink-active,
    .MuiIconButton-root {
        color: ${adminTheme.palette.brandColor};
    }
`

const StyledIconButton = styled(IconButton)`
    color: ${adminTheme.palette.brandColor};
    font-size: 24px;
    margin-left: 19px;
`

const StyledMenu = styled(Menu)`
    background-color: ${adminTheme.palette.white};
    color: ${adminTheme.palette.brandColor};
    margin-top: 0;
    margin-right: 4px;
    box-shadow: 0px 2px 1px -1px rgba(0, 0, 0, 0.2), 0px 1px 1px 0px rgba(0, 0, 0, 0.14),
        0px 1px 3px 0px rgba(0, 0, 0, 0.12);
    border-radius: 4px;
`

const DrawerContainer = styled(Box)`
    padding: 8px 16px;
    justify-content: right;
    border-top: 2px solid ${adminTheme.palette.customGrey.light};
    display: flex;
    margin-top: auto;
`

const MenuWrapper = styled(Box)`
    border-bottom: 2px solid ${adminTheme.palette.customGrey.light};
`

const CustomerSelector: React.FC = () => {
    const [open] = useSidebarState()
    const [tenantId, setTenantId] = useTenantStore()

    const {data, total, isLoading, error} = useGetList("sequent_backend_tenant", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "updated_at", order: "DESC"},
        filter: {is_active: true},
    })

    const showCustomers = open && !isLoading && !error && !!data

    const hasSingle = total === 1

    const handleChange = (event: SelectChangeEvent<unknown>) => {
        const tenantId: string = event.target.value as string
        setTenantId(tenantId)
    }

    return (
        <HorizontalBox sx={{alignItems: "center", padding: "0 16px"}}>
            <IconButton icon={faThLarge} fontSize="24px" />
            {showCustomers && (
                <>
                    {hasSingle ? (
                        <p className="ml-2.5">{data[0].username}</p>
                    ) : (
                        <Select
                            labelId="tenant-select-label"
                            id="tenant-select"
                            value={tenantId}
                            onChange={handleChange}
                            sx={{
                                flexGrow: 2,
                                paddingRight: "16px",
                                margin: "4px 10px 4px 10px",
                            }}
                        >
                            {data?.map((tenant) => (
                                <MenuItem key={tenant.id} value={tenant.id}>
                                    {tenant.username}
                                </MenuItem>
                            ))}
                        </Select>
                    )}
                    <Link to="/sequent_backend_tenant/create">
                        <StyledIconButton icon={faPlusCircle} />
                    </Link>
                </>
            )}
        </HorizontalBox>
    )
}

export const CustomMenu = () => {
    const [open, setOpen] = useSidebarState()
    const resource = useResourceContext()
    const [search, setSearch] = useState<string | null>(null)

    useEffect(() => {
        console.log("LS -> src/components/CustomMenu.tsx:128 -> resource: ", resource)
    }, [resource])

    const handleSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        // Update the state with the input field's current value
        setSearch(event.target.value)
    }

    return (
        <StyledMenu
            sx={{
                "flex": "display",
                "flexDirection": "column",
                ".RaMenuItemLink-active": {
                    backgroundColor: adminTheme.palette.green.light,
                },
            }}
        >
            <MenuWrapper>
                <CustomerSelector />
                <StyledItem
                    to="/sequent_backend_election_event"
                    primaryText={open && "Election Events"}
                    leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                />
                <HorizontalBox sx={{margin: "2px 16px"}}>
                    <Box sx={{margin: "-16px 0"}}>
                        <TextField
                            label="Search"
                            size="small"
                            value={search}
                            onChange={handleSearchChange}
                        />
                    </Box>
                    <IconButton icon={faSearch} fontSize="18px" sx={{margin: "0 12px"}} />
                </HorizontalBox>
                <TreeMenu isOpen={open} />
                <StyledItem
                    to="/pgaudit"
                    primaryText="PG Audit"
                    leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                />
                <StyledItem
                    to="/sequent_backend_area"
                    primaryText={open ? "Areas" : null}
                    leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                />
                <StyledItem
                    to="/sequent_backend_area_contest"
                    primaryText={open ? "Area Contests" : null}
                    leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                />
                <StyledItem
                    to="/sequent_backend_ballot_style"
                    primaryText={open ? "Ballot Styles" : null}
                    leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                />
                <StyledItem
                    to="/sequent_backend_tenant"
                    primaryText={open ? "Customers" : null}
                    leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                />
                <StyledItem
                    to="/sequent_backend_document"
                    primaryText={open ? "Documents" : null}
                    leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
                />
                <StyledItem
                    to="/sequent_backend_trustee"
                    primaryText={open ? "Trustees" : null}
                    leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
                />
                <StyledItem
                    to="/user-roles"
                    primaryText={open ? "User and Roles" : null}
                    leftIcon={<IconButton icon={faUsers} fontSize="24px" />}
                />
                <StyledItem
                    to="/settings"
                    primaryText={open ? "Settings" : null}
                    leftIcon={<IconButton icon={faCog} fontSize="24px" />}
                />
                <StyledItem
                    to="/messages"
                    primaryText={open ? "Messages" : null}
                    leftIcon={<IconButton icon={faStar} fontSize="24px" />}
                />
            </MenuWrapper>
            <DrawerContainer>
                <IconButton
                    icon={open ? faAngleDoubleLeft : faAngleDoubleRight}
                    fontSize="24px"
                    onClick={() => setOpen(!open)}
                />
            </DrawerContainer>
        </StyledMenu>
    )
}
