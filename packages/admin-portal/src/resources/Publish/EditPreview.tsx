import React, { useContext, useEffect, useMemo, useState } from "react";
import {
  AutocompleteInput,
  Identifier,
  SaveButton,
  SimpleForm,
  useNotify,
} from "react-admin";
import {Preview} from "@mui/icons-material"
import { useTranslation } from "react-i18next";
import { GetBallotPublicationChangesOutput, GetUploadUrlMutation } from "@/gql/graphql";
import { SettingsContext } from "@/providers/SettingsContextProvider";
import { useMutation, useQuery } from "@apollo/client";
import { GET_AREAS } from "@/queries/GetAreas";
import { GET_UPLOAD_URL } from "@/queries/GetUploadUrl";

interface EditPreviewProps {
  id?: string | Identifier | null
  electionEventId: Identifier | undefined;
  close?: () => void;
  ballotData: GetBallotPublicationChangesOutput | null;
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
  const {id, close, electionEventId, ballotData} = props
  const { t } = useTranslation();
  const notify = useNotify();
  const {globalSettings} = useContext(SettingsContext);
  const [sourceAreas, setSourceAreas] = useState([]);
  const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
  const [isUploading, setIsUploading] = React.useState<boolean>(false)
  const [documentId, setDocumentId] = React.useState<string | null>(null)
  
  const {data: areas} = useQuery(GET_AREAS, {
    variables: {
        electionEventId,
    },
  })

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

        await uploadFile(data.get_upload_url.url, theFile);
        setDocumentId(data.get_upload_url.document_id);
        notify(t("electionEventScreen.import.fileUploadSuccess"), {type: "success"})
    } catch (_error) {
        setIsUploading(false)
        notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
    }
  }

  const areaIds = useMemo(() => {
    const areaIds = ballotData?.current?.ballot_styles?.map((style: any) => ({
      id: style.area_id
    })) || [];

    return areaIds;
  }, [ballotData]);

  useEffect(() => {
    if (areas) {
      const filtered = areas.sequent_backend_area.filter((area:any) =>
        areaIds.some((areaId:any) => areaId.id === area.id)
      );
      setSourceAreas(filtered);
    }
  }, [areas, areaIds])

  const onPreviewClick = (res: any) => {
    const dataStr = JSON.stringify(ballotData, null, 2);
    const file = new File([dataStr], `${id}.json`, { type: 'application/json' });
    uploadFileToS3(file);

    const previewUrl: string = `${previewUrlTemplate}/${documentId}/${res.area_id}`;
    window.open(previewUrl, '_blank');
    notify(t("publish.previewSuccess"), { type: "success" });
    if (close) {
      close();
    }
  };

  const previewUrlTemplate = useMemo(() => {
    return `${globalSettings.VOTING_PORTAL_URL}/preview`;
  }, [globalSettings.VOTING_PORTAL_URL, id])

  return (
      <SimpleForm
        onSubmit={onPreviewClick}
        toolbar={
          <SaveButton 
            icon={<Preview />}
            label={t("publish.preview")}
            sx={{marginInline: "1rem"}}
          />
        }
      >
        <AutocompleteInput
          source="area_id"
          choices={sourceAreas} 
          optionText={(area) => area.name}
          label={t("publis.publicationAreas")}
          fullWidth={true}
          debounce={100}>
        </AutocompleteInput>
        
      </SimpleForm>
  );
};
