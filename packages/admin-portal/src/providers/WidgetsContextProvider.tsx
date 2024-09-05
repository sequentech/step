import {Widget, WidgetStateProps} from "@/components/Widget"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {useQuery} from "@apollo/client"
import React, {createContext, useContext, useEffect, useState} from "react"
import {SettingsContext} from "./SettingsContextProvider"
import { ETasksExecution } from "@/types/tasksExecution"
import { ETaskExecutionStatus } from "@sequentech/ui-core"

interface WidgetContextProps {
    widgetState: WidgetStateProps | undefined
    setWidgetState: (val: WidgetStateProps | undefined) => void
    taskId: string | undefined
    setTaskId: (val: string | undefined) => void
}

const WidgetContext = createContext<WidgetContextProps>({
    widgetState: undefined,
    setWidgetState: () => {},
    taskId: undefined,
    setTaskId: () => {},
})

export const WidgetsContextProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const {globalSettings} = useContext(SettingsContext)
    const [widgetState, setWidgetState] = useState<WidgetStateProps | undefined>(undefined)
    const [taskId, setTaskId] = useState<string | undefined>(undefined)

    const {data: taskData} = useQuery(GET_TASK_BY_ID, {
        variables: {task_id: taskId},
        skip: !taskId,
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    useEffect(() => {
        console.log("task data", taskData)
        if (taskData) {
            setWidgetState({
                type: taskData?.sequent_backend_tasks_execution[0].type,
                status: taskData?.sequent_backend_tasks_execution[0].execution_status,
                logs: taskData?.sequent_backend_tasks_execution[0].logs,
                id: taskId,
            })
        }
    }, [taskData])

    const onCloseWidget = () => {
        setWidgetState(undefined)
        setTaskId(undefined)
    }

    return (
        <WidgetContext.Provider value={{widgetState, setWidgetState, taskId: taskId, setTaskId}}>
            {children}
            {widgetState && (
                <Widget
                    type={widgetState.type|| ETasksExecution.EXPORT_ELECTION_EVENT}
                    status={widgetState.status || ETaskExecutionStatus.CANCELLED}
                    logs={widgetState.logs}
                    onClose={onCloseWidget}
                    id={taskId}
                />
            )}
        </WidgetContext.Provider>
    )
}

export const useWidgetStore = () => {
    const {widgetState, setWidgetState, taskId, setTaskId} = useContext(WidgetContext)
    return [widgetState, setWidgetState, taskId, setTaskId] as const
}
