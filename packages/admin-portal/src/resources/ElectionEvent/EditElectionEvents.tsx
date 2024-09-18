import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Button, Typography} from "@mui/material"
import React, {ReactElement, useContext} from "react"
import {
    BooleanField,
    DatagridConfigurable,
    FunctionField,
    List,
    TextField,
    TextInput,
    WrapperField,
} from "react-admin"
import {JsonField} from "react-admin-json-view"
import {useTranslation} from "react-i18next"

const EditElectionEvents = () => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)

    const OMIT_FIELDS: Array<string> = ["id", "email_verified"]

    const Filters: Array<ReactElement> = [
        <TextInput key="email" source="email" />,
        <TextInput key="first_name" source="first_name" />,
        <TextInput key="last_name" source="last_name" />,
        <TextInput key="username" source="username" />,
    ]

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

    return (
        <List
            resource="sequent_backend_scheduled_event"
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
            filter={
                {
                    // tenant_id: tenantId,
                    // election_event_id: electionEventId,
                    // election_id: electionId,
                }
            }
            // aside={aside}
            filters={Filters}
        >
            <DatagridConfigurable
                omit={OMIT_FIELDS}
                // bulkActionButtons={<BulkActions />}
            >
                <TextField source="event_processor" label={"Election"} />
                {/* <JsonField source="cron_config" className="email" label={"Schedule"} /> */}
                <FunctionField
                    source="cron_config"
                    label={"Schedule"}
                    render={(record: any) => (
                        <Typography>
                            {new Date(record?.cron_config.scheduled_date).toLocaleString()}
                        </Typography>
                    )}
                />
                <BooleanField source="email_verified" label={"Event Type"} />
                <BooleanField source="enabled" />
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

export default EditElectionEvents
