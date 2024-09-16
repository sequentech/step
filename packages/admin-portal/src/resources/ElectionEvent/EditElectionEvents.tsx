// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Sequent_Backend_Election, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Typography} from "@mui/material"
import React, {ReactElement} from "react"
import {
    BooleanField,
    BulkActionsToolbar,
    DatagridConfigurable,
    FunctionField,
    List,
    TextField,
    TextInput,
    useRecordContext,
    WrapperField,
} from "react-admin"
import {useTranslation} from "react-i18next"

const Filters: Array<ReactElement> = [
    <TextInput key="email" source="email" />,
    <TextInput key="first_name" source="first_name" />,
    <TextInput key="last_name" source="last_name" />,
    <TextInput key="username" source="username" />,
]

const EditElectionEvents = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election>()
    // const {globalSettings} = React.useContext(SettingsContext)
    const electionEventId = record?.id
    // const electionId = record?.election_event_id
    const [tenantId] = useTenantStore()
    const OMIT_FIELDS: Array<string> = ["id", "email_verified"]

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`usersAndRolesScreen.${record.id ? "voters" : "users"}.emptyHeader`)}
            </Typography>
            {/* {canEditUsers ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.askCreate`)}
                    </Typography>
                    <ResourceListStyles.EmptyButtonList className="voter-add-button">
                        <Button onClick={() => setOpenNew(true)}>
                            <ResourceListStyles.CreateIcon icon={faPlus} />
                            {t(
                                `usersAndRolesScreen.${
                                    electionEventId ? "voters" : "users"
                                }.create.subtitle`
                            )}
                        </Button>
                        <ReactAdminButton onClick={handleImport} label={t("common.label.import")}>
                            <UploadIcon />
                        </ReactAdminButton>
                    </ResourceListStyles.EmptyButtonList>
                </>
            ) : null} */}
        </ResourceListStyles.EmptyBox>
    )

    return (
        <List
            resource="sequent_backend_election"
            // queryOptions={{
            //     refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            // }}
            empty={<Empty />}
            actions={
                <ListActions
                    // withImport
                    // doImport={handleImport}
                    withExport
                    // doExport={handleExport}
                    // open={openDrawer}
                    // setOpen={setOpenDrawer}
                    // Component={<CreateUser electionEventId={electionEventId} close={handleClose} />}
                    //         extraActions={[
                    //             <Button
                    //                 key="send-notification"
                    //                 onClick={() => {
                    //                     sendCommunicationAction([], AudienceSelection.ALL_USERS)
                    //                 }}
                    //             >
                    //                 <ResourceListStyles.MailIcon />
                    //                 {t("sendCommunication.send")}
                    //             </Button>,
                    //         ]}
                />
            }
            filter={{
                tenant_id: tenantId,
                election_event_id: electionEventId,
                // election_id: electionId,
            }}
            // aside={aside}
            filters={Filters}
        >
            <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<BulkActionsToolbar />}>
                <TextField source="id" />
                <TextField
                    source="name"
                    className="name"
                    label={t("usersAndRolesScreen.common.mobileNumber")}
                />
                <BooleanField source="email_verified" />
                <BooleanField source="enabled" />
                <TextField source="first_name" className="first_name" />
                <TextField
                    label={t("usersAndRolesScreen.common.mobileNumber")}
                    source="attributes['sequent.read-only.mobile-number']"
                />
                <TextField source="last_name" className="last_name" />
                <TextField source="username" className="username" />
                {/* {electionEventId && (
                    <FunctionField
                        label={t("usersAndRolesScreen.users.fields.area")}
                        render={(record: IUser) =>
                            record?.area?.name ? <Chip label={record?.area?.name ?? ""} /> : "-"
                        }
                    />
                )} */}
                {/* {electionEventId && (
                    <FunctionField
                        source="has_voted"
                        label={t("usersAndRolesScreen.users.fields.has_voted")}
                        render={<div></div>}
                        // render={(record: IUser, source: string | undefined) => {
                        //     let newRecord = {
                        //         has_voted: (record?.votes_info?.length ?? 0) > 0,
                        //         ...record,
                        //     }
                        //     return <BooleanField record={newRecord} source={source} />
                        // }}
                    />
                )} */}
                <WrapperField source="actions" label="Actions">
                    <div></div>
                    {/* <ActionsColumn actions={actions} /> */}
                </WrapperField>
            </DatagridConfigurable>
        </List>
    )
}

export default EditElectionEvents
