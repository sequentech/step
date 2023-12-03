import React from "react"
import {CreateBase, EditBase, Identifier, RaRecord} from "react-admin"
import {CandidateDataForm, Sequent_Backend_Candidate_Extended} from "./CandidateDataForm"

export const CreateCandidateData: React.FC = () => {
    const transform = (data: Sequent_Backend_Candidate_Extended): RaRecord<Identifier> => {

        // save presentation object
        // language_conf
        console.log("data before :: ", data)
        const enabled_language_codes = []
        for (const key in data.enabled_languages) {
            if (typeof data.enabled_languages[key] === "boolean" && data.enabled_languages[key]) {
                enabled_language_codes.push(key)
            }
        }
        const language_conf = {
            enabled_language_codes: enabled_language_codes,
        }
        // i18n
        // is alll object, no change needed
        delete data.enabled_languages

        return {
            ...data,
            presentation: {
                ...data.presentation,
                language_conf: {
                    ...language_conf,
                    default_language_code: data.defaultLanguage,
                },
            },
        }
    }

    return (
        <CreateBase redirect={"show"} transform={transform}>
            <CandidateDataForm />
        </CreateBase>
    )
}
