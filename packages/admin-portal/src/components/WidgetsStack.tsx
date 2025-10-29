// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Widget, WidgetProps} from "./Widget"
import {StackContainer} from "./styles/WidgetStyle"

interface WidgetsStackProps {
    widgetsMap: Map<string, WidgetProps>
}

export const WidgetsStack: React.FC<WidgetsStackProps> = ({widgetsMap}) => {
    const widgets: WidgetProps[] = Array.from(widgetsMap.values())

    return (
        <StackContainer className="widget-stack">
            {widgets.map((widget) => (
                <Widget
                    key={widget.identifier}
                    identifier={widget.identifier}
                    type={widget.type}
                    status={widget.status}
                    logs={widget.logs}
                    onClose={() => widget.onClose(widget.identifier)}
                    taskId={widget.taskId}
                    onSuccess={widget.onSuccess}
                    automaticallyDownload={widget.automaticallyDownload}
                />
            ))}
        </StackContainer>
    )
}
