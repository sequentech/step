// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Button, Typography} from "@mui/material"
import React, {ReactElement, useContext, useEffect} from "react"
import {
    AuthContext,
    BooleanField,
    DatagridConfigurable,
    FunctionField,
    List,
    TextField,
    TextInput,
    useRecordContext,
    WrapperField,
} from "react-admin"
import {JsonField} from "react-admin-json-view"
import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
// import {GetEventListMutation} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {CustomApolloContextProvider} from "@/providers/ApolloContextProvider"

const EditEvents = () => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext()
    // const [eventList] = useMutation<GetEventListMutation>(GET_EVENT_LIST, {
    //     context: {
    //         headers: {
    //             "x-hasura-role": IPermissions.ELECTION_EVENT_READ,
    //         },
    //     },
    // })
    console.log(record, "record")

    const [tenantId] = useTenantStore()

    const OMIT_FIELDS: Array<string> = ["id", "email_verified"]

    const Filters: Array<ReactElement> = [
        <TextInput key="Election" source="election" />,
        <TextInput key="EventType" source="event_type" />,
        <TextInput key="Name" source="name" />,
        <TextInput key="Schedule" source="schedule" />,
    ]

    // useEffect(() => {
    //     const fetchData = async () => {
    //         if (tenantId && record.id) {
    //             const {data} = await eventList({
    //                 variables: {tenantId, electionEventId: record.id},
    //             })
    //             console.log(data)
    //         }
    //     }
    //     fetchData()
    // }, [tenantId, eventList, record.id])
    const actions: Action[] = [
        // {
        //     icon: <MailIcon />,
        //     action: sendCommunicationForIdAction,
        //     showAction: () => canSendCommunications,
        // },
        // {
        //     icon: <EditIcon className="edit-voter-icon" />,
        //     action: editAction,
        //     showAction: () => canEditUsers,
        // },
        // {
        //     icon: <DeleteIcon className="delete-voter-icon" />,
        //     action: deleteAction,
        //     showAction: () => canEditUsers,
        //     className: "delete-voter-icon",
        // },
        // {
        //     icon: <CreditScoreIcon />,
        //     action: manualVerificationAction,
        //     showAction: () => canEditUsers,
        // },
    ]
    const filterObject: {[key: string]: any} = {
        election_event_id: record?.id || undefined,
    }
    return (
        <List
            resource="event_list"
            queryOptions={{
                refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            }}
            // empty={<Empty />}
            actions={
                <ListActions
                    withImport
                    // doImport={handleImport}
                    withExport
                    // doExport={handleExport}
                    // open={openDrawer}
                    // setOpen={setOpenDrawer}
                    // Component={<CreateUser electionEventId={electionEventId} close={handleClose} />}
                    extraActions={[
                        <Button
                            key="send-notification"
                            onClick={() => {
                                console.log("test")
                            }}
                            // onClick={() => {
                            //     sendCommunicationAction([], AudienceSelection.ALL_USERS)
                            // }}
                        >
                            <ResourceListStyles.MailIcon />
                            {t("sendCommunication.send")}
                        </Button>,
                    ]}
                />
            }
            filter={{
                tenant_id: tenantId,
                election_event_id: record.id,
            }}
            // aside={aside}
            filters={Filters}
        >
            <DatagridConfigurable
                omit={OMIT_FIELDS}
                // bulkActionButtons={<BulkActions />}
            >
                <TextField source="election" label={"Election"} />
                <TextField source="name" label={"Task Name"} />
                <TextField source="schedule" label={"Schedule"} />
                <TextField source="event_type" label={"Event Type"} />
                {/* <JsonField source="cron_config" className="email" label={"Schedule"} /> */}
                {/* <FunctionField
                    source="cron_config"
                    label={"Event Type"}
                    render={(record: any) => (
                        <Typography>
                            {new Date(record?.cron_config.scheduled_date).toLocaleString()
                                ? "event"
                                : ""}
                        </Typography>
                    )}
                /> */}
                <BooleanField source="receivers" label={"Receivers"} />
                <TextField source="template" label={"Template"} />
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
                        render={(record: IUser, source: string | undefined) => {
                            let newRecord = {
                                has_voted: (record?.votes_info?.length ?? 0) > 0,
                                ...record,
                            }
                            return <BooleanField record={newRecord} source={source} />
                        }}
                    />
                )} */}
                <WrapperField source="actions" label="Actions">
                    <ActionsColumn actions={actions} />
                </WrapperField>
            </DatagridConfigurable>
        </List>
    )
}

export default EditEvents
