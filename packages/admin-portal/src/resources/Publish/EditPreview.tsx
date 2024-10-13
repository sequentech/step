import React, { useContext, useEffect, useMemo, useState } from "react";
import {
  AutocompleteInput,
  Identifier,
  SaveButton,
  SimpleForm,
  useNotify,
  useRedirect,
} from "react-admin";
import {Preview} from "@mui/icons-material"
import { useTranslation } from "react-i18next";
import { useTenantStore } from "@/providers/TenantContextProvider";
import { GetBallotPublicationChangesOutput } from "@/gql/graphql";
import SelectArea from "@/components/area/SelectArea";
import { SettingsContext } from "@/providers/SettingsContextProvider";
import { useQuery } from "@apollo/client";
import { GET_AREAS } from "@/queries/GetAreas";

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
  const [tenantId] = useTenantStore();
  const {globalSettings} = useContext(SettingsContext);
  const [sourceAreas, setSourceAreas] = useState([])
  const redirect = useRedirect();
  
  const {data: areas} = useQuery(GET_AREAS, {
    variables: {
        electionEventId,
    },
  })

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
    notify(t("publish.previewSuccess"), { type: "success" });
    if (close) {
      close();
    }
  };

  const previewUrl = useMemo(() => {
    return globalSettings.VOTING_PORTAL_URL + "/preview/" + id;
  }, [globalSettings.VOTING_PORTAL_URL, id])

  return (
      <SimpleForm
        toolbar={
          <SaveButton 
            icon={<Preview />}
            label={t("publish.preview")}
            sx={{marginInline: "1rem"}}
            onClick={onPreviewClick}
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
