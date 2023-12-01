// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {
    Box,
    SelectChangeEvent,
    Typography,
} from "@mui/material"
import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Trustee,
    CreateScheduledEventMutation,
} from "../gql/graphql"
import {
    useGetList,
    useRefresh,
    SimpleForm,
    TextInput,
    Toolbar,
    SaveButton,
    CheckboxGroupInput,
} from "react-admin"
import {styled} from "@mui/material/styles"
import {useMutation} from "@apollo/client"
import {CREATE_SCHEDULED_EVENT} from "../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../services/ScheduledEvent"
import { useTranslation } from "react-i18next"
import { isNumber } from "lodash"

const Horizontal = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 8px;
`

const StyledToolbar = styled(Toolbar)`
    flex-direction: row-reverse;
`

const CreateButton = styled(SaveButton)``

export interface KeysGenerationStepProps {
    onCreate: (index: number) => void
    electionEvent: Sequent_Backend_Election_Event
}

export const KeysGenerationStep: React.FC<KeysGenerationStepProps> = ({
    onCreate: onCreate,
    electionEvent,
}) => {
    const {t} = useTranslation()
    const [selectedTrustees, setSelectedTrustees] = useState<Array<Sequent_Backend_Trustee>>([])
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const [threshold, setThreshold] = useState(2)
    const [trustee, setTrustee] = useState<Sequent_Backend_Trustee | null>(null)
    const refresh = useRefresh()
    const {data: trusteeList, total, isLoading, error} = useGetList(
        "sequent_backend_trustee",
        {
            pagination: {page: 1, perPage: 10},
            sort: {field: "last_updated_at", order: "DESC"},
            filter: {
                tenant_id: electionEvent.tenant_id,
            },
        }
    )

    if (isLoading || error) {
        return null
    }

    const generateKeys = async () => {
        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: electionEvent.tenant_id,
                electionEventId: electionEvent.id,
                eventProcessor: ScheduledEventType.CREATE_KEYS,
                cronConfig: undefined,
                eventPayload: {
                    trustee_pks: selectedTrustees.map((trustee) => trustee.public_key),
                    threshold: threshold,
                },
                createdBy: "admin",
            },
        })
        if (errors) {
            console.log(errors)
        }
        if (data) {
            console.log(data)
        }
        refresh()
    }

    const onCreateHandler = async () => {
        try {
            await generateKeys()
            onCreate(0)
        } catch (error) {
            console.log(`Error trying to create keys: ${error}`)
        }
    }

    const getDefaultValues = () => ({
        threshold: 2
    })

    const thresholdValidator = (value: any): any => {
        const max = (trusteeList) ? trusteeList.length : 0
        var intValue: number | null = null
        try {
            intValue = parseInt(value)
        } catch {
            intValue = null
        }
        if (
            !isNumber(intValue) ||
            isNaN(intValue) ||
            intValue < 2 ||
            intValue > max
        ) {
            console.log(`thresholdValidator: error, intValue=${intValue}`)
            return t(
                "keysGenerationStep.errorThreshold",
                {selected: intValue, min: 0, max: max}
            )
        }
        console.log(`thresholdValidator: NO error, intValue=${intValue}`)
    }

    const trusteeListValidator = (values: any): any => {
        const length = (values && values.length) ? values.length : 0
        if (length < threshold) {
            return t(
                "keysGenerationStep.errorMinTrustees",
                {selected: length, threshold: threshold}
            )
        }
    }

    return (
        <SimpleForm
            defaultValues={getDefaultValues}
            onSubmit={onCreateHandler}
            toolbar={
                <StyledToolbar>
                    <CreateButton label={t("keysGenerationStep.create")} />
                </StyledToolbar>
            }
        >
            <Typography variant="h4">
                {t("keysGenerationStep.title")}
            </Typography>
            <TextInput
                source="threshold"
                value={threshold}
                label={t("keysGenerationStep.threshold")}
                validate={thresholdValidator}
                type="number"
                InputLabelProps={{
                    shrink: true,
                }}
                variant="filled"
            />
            {trusteeList ? (
                <CheckboxGroupInput
                    validate={trusteeListValidator}
                    label={t("keysGenerationStep.trusteeList")}
                    source="selectedTrustees"
                    choices={trusteeList}
                    translateChoice={false}
                    optionText="name"
                    optionValue="id"
                    row={false}
                />
            ) : null}
        </SimpleForm>
    )
}
