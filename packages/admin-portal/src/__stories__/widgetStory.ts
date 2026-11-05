// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {ComponentType} from "react"
import type {Meta} from "@storybook/react-vite"

/**
 * Meta of a widget section whose stories take a scenario rather than the
 * widget's own props: `component` names the production widget, which the
 * catalog inventory and the rendered-widget check use, and `args` are the
 * scenario's. Type its stories with `StoryObj<Scenario>`.
 */
export type WidgetMeta<Args> = Omit<Meta<Args>, "component"> & {component: ComponentType<never>}
