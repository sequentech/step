import React, {useContext, useEffect, useMemo, useState} from "react"
import {
    AutocompleteInput,
    Identifier,
    SaveButton,
    SimpleForm,
    useGetList,
    useGetOne,
    useNotify,
} from "react-admin"
import {Preview} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {
    GetBallotPublicationChangesOutput,
    GetUploadUrlMutation,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useMutation, useQuery} from "@apollo/client"
import {GET_AREAS} from "@/queries/GetAreas"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {TenantContext} from "@/providers/TenantContextProvider"

interface EditPreviewProps {
    id?: string | Identifier | null
    electionEventId: Identifier | undefined
    close?: () => void
    ballotData: GetBallotPublicationChangesOutput | null
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
    const {id, close, electionEventId, ballotData} = props
    const {t} = useTranslation()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const [sourceAreas, setSourceAreas] = useState([])
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const [isUploading, setIsUploading] = React.useState<boolean>(false)
    const {tenantId} = useContext(TenantContext)
    const [areaId, setAreaId] = useState<string | null>(null)

    const {data: areas} = useQuery(GET_AREAS, {
        variables: {
            electionEventId,
        },
    })

    const {data: electionEvent} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            id: electionEventId,
        },
        {
            refetchIntervalInBackground: true,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: elections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 9999},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                election_event_id: electionEventId,
                tenant_id: tenantId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const uploadFile = async (url: string, file: File) => {
        await fetch(url, {
            method: "PUT",
            headers: {
                "Content-Type": file.type,
            },
            body: file,
        })
        setIsUploading(false)
    }

    const uploadFileToS3 = async (theFile: File) => {
        try {
            let {data} = await getUploadUrl({
                variables: {
                    name: theFile.name,
                    media_type: theFile.type,
                    size: theFile.size,
                    is_public: true,
                },
            })

            if (!data?.get_upload_url?.url) {
                notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
                return
            }

            await uploadFile(data.get_upload_url.url, theFile)
            notify(t("electionEventScreen.import.fileUploadSuccess"), {type: "success"})
            return data.get_upload_url.document_id
        } catch (_error) {
            setIsUploading(false)
            notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
        }
    }

    const areaIds = useMemo(() => {
        const areaIds =
            ballotData?.current?.ballot_styles?.map((style: any) => ({
                id: style.area_id,
            })) || []

        return areaIds
    }, [ballotData])

    useEffect(() => {
        if (areas) {
            const filtered = areas.sequent_backend_area.filter((area: any) =>
                areaIds.some((areaId: any) => areaId.id === area.id)
            )
            setSourceAreas(filtered)
        }
    }, [areas, areaIds])

    useEffect(() => {
        const startUpload = async () => {
            const fileData = {
                ballot_styles: ballotData?.current?.ballot_styles,
                election_event: electionEvent,
                elections: elections,
            }
            const dataStr = JSON.stringify(fileData, null, 2)
            const file = new File([dataStr], `preview.json`, {type: "application/json"})
            const documentId = await uploadFileToS3(file)

            const previewUrl: string = `${previewUrlTemplate}/${documentId}/${areaId}`
            window.open(previewUrl, "_blank")

            notify(t("publish.preview.success"), {type: "success"})
            if (close) {
                close()
            }
        }
        if (isUploading && electionEvent && elections && areaId) {
            startUpload()
        }
    }, [isUploading, electionEvent, elections, areaId])

    const onPreviewClick = async (res: any) => {
        setAreaId(res.area_id)
        setIsUploading(true)
    }

    const previewUrlTemplate = useMemo(() => {
        return `${globalSettings.VOTING_PORTAL_URL}/preview/${tenantId}`
    }, [globalSettings.VOTING_PORTAL_URL, id])

    return (
        <SimpleForm
            onSubmit={onPreviewClick}
            toolbar={
                <SaveButton
                    icon={<Preview />}
                    label={t("publish.preview.action")}
                    sx={{marginInline: "1rem"}}
                />
            }
        >
            <AutocompleteInput
                source="area_id"
                choices={sourceAreas}
                optionText={(area) => area.name}
                label={t("publish.preview.publicationAreas")}
                fullWidth={true}
                debounce={100}
            ></AutocompleteInput>
        </SimpleForm>
    )
}
