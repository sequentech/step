// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useContext, useEffect, useState} from "react"
import {ETasksExecution} from "@/types/tasksExecution"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {WidgetsStack} from "@/components/WidgetsStack"
import {WidgetProps} from "@/components/Widget"

interface WidgetContextProps {
    addWidget: (type: ETasksExecution) => WidgetProps
    setWidgetTaskId: (widgetIdentifier: string, taskId: string, onSuccess?: () => void) => void
    updateWidgetFail: (widgetIdentifier: string) => void
}

const WidgetContext = createContext<WidgetContextProps>({
    addWidget: () => ({} as WidgetProps),
    setWidgetTaskId: () => {},
    updateWidgetFail: () => {},
})

export const WidgetsContextProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const [widgetsState, setWidgetsState] = useState<Map<string, WidgetProps>>(new Map())

    const generateWidgetId = (): string => {
        const now = new Date()
        const formattedDate = now.toISOString().replace(/[-:.TZ]/g, "")
        return `widget_${formattedDate}`
    }

    const addWidget = (type: ETasksExecution): WidgetProps => {
        const newWidget: WidgetProps = {
            type,
            status: ETaskExecutionStatus.IN_PROGRESS,
            onClose: onClose,
            identifier: generateWidgetId(),
        }

        setWidgetsState((prevState) => new Map(prevState.set(newWidget.identifier, newWidget)))
        return newWidget
    }

    const setWidgetTaskId = (widgetIdentifier: string, taskId: string, onSuccess?: () => void) => {
        setWidgetsState((prevState) => {
            const widget = prevState.get(widgetIdentifier)
            if (widget) {
                widget.taskId = taskId
                widget.onSuccess = onSuccess
                prevState.set(widgetIdentifier, {...widget})
            }
            return new Map(prevState)
        })
    }

    const updateWidgetFail = (widgetIdentifier: string) => {
        setWidgetsState((prevState) => {
            const widget = prevState.get(widgetIdentifier)
            if (widget) {
                widget.status = ETaskExecutionStatus.FAILED
                prevState.set(widgetIdentifier, widget)
            }
            return new Map(prevState)
        })
    }

    const onClose = (widgetIdentifier: string) => {
        setWidgetsState((prevState) => {
            prevState.delete(widgetIdentifier)
            return new Map(prevState)
        })
    }

    return (
        <WidgetContext.Provider value={{addWidget, setWidgetTaskId, updateWidgetFail}}>
            {children}
            {widgetsState.size > 0 && <WidgetsStack widgetsMap={widgetsState} />}
        </WidgetContext.Provider>
    )
}

export const useWidgetStore = () => {
    const {addWidget, setWidgetTaskId, updateWidgetFail} = useContext(WidgetContext)
    return [addWidget, setWidgetTaskId, updateWidgetFail] as const
}
