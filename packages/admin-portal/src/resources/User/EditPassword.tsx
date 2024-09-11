import React, { useState } from "react";
import Dialog from "@mui/material/Dialog"
import DialogTitle from "@mui/material/DialogTitle"
import { useTranslation } from "react-i18next";
import { BooleanInput, Edit, PasswordInput, SaveButton, SimpleForm, useNotify, useRefresh } from "react-admin";
import { DialogContent, InputLabel, Modal } from "@mui/material";
import { IUser } from "sequent-core";
import { useMutation } from "@apollo/client";
import { EditUsersInput } from "@/gql/graphql";
import { EDIT_USER } from "@/queries/EditUser";
import { FormStyles } from "@/components/styles/FormStyles";
import { faTimesCircle } from "@fortawesome/free-solid-svg-icons";
import { IconButton } from "@sequentech/ui-essentials";

interface EditPasswordProps {
    open: boolean;
    handleClose: () => void;
    userId: string;
}

const EditPassword = ({ open, handleClose, userId }: EditPasswordProps) => {
    const { t } = useTranslation()
    const [user, setUser] = useState<IUser>({ id: userId, password: '' })
    const refresh = useRefresh()
    const notify = useNotify()
    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)

    const equalToPassword = (value: any, allValues: any) => {
        console.log("value", value, allValues);

        if (!allValues.password || allValues.password.length == 0) {
            return
        }
        if (value !== allValues.password) {
            return t("usersAndRolesScreen.users.fields.passwordMismatch")
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        e.preventDefault();
        e.stopPropagation();
        const { name, value } = e.target
        let newUser = { ...user, [name]: value }
        console.log(`newUser = `)
        console.log(newUser)
        setUser(newUser)
    }

    const onSubmit = async () => {
        try {
            let { data } = await edit_user({
                variables: {
                    body: {
                        user_id: user?.id,
                        tenant_id: "",
                        election_event_id: "electionEventId",
                        password:
                            user?.password && user?.password.length > 0
                                ? user.password
                                : undefined,
                    },
                },
            })
            notify(t("usersAndRolesScreen.voters.errors.editSuccess"), { type: "success" })
            refresh()
            handleClose?.()
        } catch (error) {
            notify(t("usersAndRolesScreen.voters.errors.editError"), { type: "error" })
            handleClose?.()
        }

    }

    return (
        <Dialog
            open={open}
            onClose={handleClose}
            className="dialog"
        >
            <DialogTitle>
                edit
                <IconButton
                    icon={faTimesCircle}
                    variant="primary"
                    onClick={() => handleClose()}
                    className="dialog-icon-close"
                />
            </DialogTitle>
            <DialogContent>
                <SimpleForm
                    toolbar={<SaveButton alwaysEnable />}
                    record={user}
                    onSubmit={() => ''}
                    sanitizeEmptyValues>
                    <>
                        <FormStyles.PasswordInput
                            label={t("usersAndRolesScreen.users.fields.password")}
                            source="password"
                            onChange={(e) => {
                                console.log("Password Input changed", e.target.value);
                                handleChange(e);
                            }}
                        />
                        <FormStyles.PasswordInput
                            label={t("usersAndRolesScreen.users.fields.repeatPassword")}
                            source="confirm_password"
                            validate={equalToPassword}
                            onChange={handleChange}
                        />

                    </>
                </SimpleForm>
            </DialogContent>
        </Dialog >
    );
};

export default EditPassword;
