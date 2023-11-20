import React from "react"
import {EditBase} from "react-admin"
import { EditElectionDataForm } from './ElectionDataForm'

export const EditElectionData: React.FC = () => {
    return (
        <EditBase redirect={"."}>
            <EditElectionDataForm />
        </EditBase>
    )
}
