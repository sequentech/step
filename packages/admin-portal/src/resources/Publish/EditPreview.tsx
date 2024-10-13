import React, { useContext, useMemo } from "react";
import {
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

interface EditPreviewProps {
  id?: string | Identifier | null
  electionEventId: Identifier | undefined;
  close?: () => void;
  data: GetBallotPublicationChangesOutput | null;
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
  const {id, close, electionEventId, data} = props
  const { t } = useTranslation();
  const notify = useNotify();
  const [tenantId] = useTenantStore();
  const {globalSettings} = useContext(SettingsContext)
  const redirect = useRedirect()

  const onPreviewClick = (res: any) => {
    console.log({data})
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
          <SelectArea
            tenantId={tenantId}
            electionEventId={electionEventId}
            source={"areas"}
          />
        </SimpleForm>
    );
};
