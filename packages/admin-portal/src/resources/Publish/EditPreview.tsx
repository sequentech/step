import React, { useContext, useEffect, useMemo, useState } from "react";
import {
  Identifier,
  SaveButton,
  SimpleForm,
  useNotify,
  useRedirect,
} from "react-admin";
import {Preview} from "@mui/icons-material"
import { useQuery } from "@apollo/client";
import { useTranslation } from "react-i18next";
import { useTenantStore } from "@/providers/TenantContextProvider";
import { GetBallotStylesQuery } from "@/gql/graphql";
import SelectArea from "@/components/area/SelectArea";
import { GET_BALLOT_STYLES } from "@/queries/GetBallotStyles";
import { SettingsContext } from "@/providers/SettingsContextProvider";

interface EditPreviewProps {
  id?: Identifier | undefined
  electionEventId: Identifier | undefined;
  close?: () => void;
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
  const {id, close, electionEventId} = props
  const { t } = useTranslation();
  const notify = useNotify();
  const [renderUI, setRenderUI] = useState(false);
  const [tenantId] = useTenantStore();
  const {globalSettings} = useContext(SettingsContext)
  const redirect = useRedirect()
  const { data: dataBallotStyles } = useQuery<GetBallotStylesQuery>(
    GET_BALLOT_STYLES
  );

  const onPreviewClick = (res: any) => {
    notify(t("publish.previewSuccess"), { type: "success" });
    console.log({res});
    if (close) {
      close();
    }
  };

  const previewUrl = useMemo(() => {
    return globalSettings.VOTING_PORTAL_URL + "/preview/" + id;
  }, [globalSettings.VOTING_PORTAL_URL, id])

  useEffect(() => {
    if (dataBallotStyles) {
      setRenderUI(true);
    }
  }, [dataBallotStyles]);

  if (renderUI) {
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
          <SelectArea
            tenantId={tenantId}
            electionEventId={electionEventId}
            source="parent_id"
          />
        </SimpleForm>
    );
  } else {
    return null;
  }
};
