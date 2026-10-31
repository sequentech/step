// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, type PropsWithChildren} from "react"
import type {Decorator, Preview} from "@storybook/react-vite"
import {expect} from "storybook/test"
import preview from "../../ui-essentials/.storybook/preview"
import {DEFAULT_STORY_GLOBALS, storyGlobalTypes} from "../../ui-essentials/.storybook/globals"
import {
    assertRenderedWidgets,
    observeRenderedWidgets,
} from "../../test-support/storybook/renderedWidgets"
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../src/translations/en"
import spanishTranslation from "../src/translations/es"
import catalanTranslation from "../src/translations/cat"
import frenchTranslation from "../src/translations/fr"
import tagalogTranslation from "../src/translations/tl"
import galegoTranslation from "../src/translations/gl"
import dutchTranslation from "../src/translations/nl"
import basqueTranslation from "../src/translations/eu"
import {guardStoryNetwork, storyViolations} from "../src/__stories__/storyNetwork"

initializeLanguages(
    {
        en: englishTranslation,
        es: spanishTranslation,
        cat: catalanTranslation,
        fr: frenchTranslation,
        tl: tagalogTranslation,
        gl: galegoTranslation,
        nl: dutchTranslation,
        eu: basqueTranslation,
    },
    DEFAULT_STORY_GLOBALS.locale
)

const observations = new Map<string, ReturnType<typeof observeRenderedWidgets>>()

/** Records the components each story renders, from its first commit until it unmounts. */
function WidgetObserver({
    id,
    canvasElement,
    component,
    children,
}: PropsWithChildren<{id: string; canvasElement: HTMLElement; component: unknown}>) {
    useEffect(() => {
        const observer = observeRenderedWidgets(canvasElement, component)
        observations.set(id, observer)
        return () => {
            observer.stop()
            if (observations.get(id) === observer) observations.delete(id)
        }
    }, [id, canvasElement, component])
    return <>{children}</>
}

// Outermost, so that its effect runs after the whole story has committed.
const withWidgetObserver: Decorator = (Story, {id, canvasElement, component}) => (
    <WidgetObserver id={id} canvasElement={canvasElement} component={component}>
        <Story />
    </WidgetObserver>
)

export default {
    ...preview,
    globalTypes: {
        ...preview.globalTypes,
        tenant: storyGlobalTypes.tenant,
        permissions: storyGlobalTypes.permissions,
        workflow: storyGlobalTypes.workflow,
    },
    decorators: [...[preview.decorators ?? []].flat(), withWidgetObserver],
    // Every story, in the Storybook UI as in the test runner, answers requests
    // through its boundaries; any other request fails like an unreachable server.
    beforeEach: [...[preview.beforeEach ?? []].flat(), () => guardStoryNetwork()],
    afterEach: [
        ...[preview.afterEach ?? []].flat(),
        ({id, component, parameters}) => {
            // Each section renders its production component and the embedded
            // widgets its stories name in `parameters.widgets`.
            const observer = observations.get(id)
            observer?.record()
            assertRenderedWidgets(observer?.observation, component, parameters.widgets)
            expect(storyViolations(), "Unexpected requests or boundary operations").toEqual([])
        },
    ],
} satisfies Preview
