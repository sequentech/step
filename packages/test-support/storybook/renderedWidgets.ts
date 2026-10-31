// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Records which components a story renders by walking React's fiber tree
// from the story's root element whenever the document changes. A widget
// catalog uses it to prove that each story renders its production component,
// and the embedded components that a story's `parameters.widgets` names,
// rather than a copy of them.
interface Fiber {
    elementType: unknown
    type: unknown
    child: Fiber | null
    sibling: Fiber | null
}

type Wrapped = {
    type?: unknown
    render?: unknown
    displayName?: string
    name?: string
}

/** The function or class that renders, looking through memo and forwardRef. */
function renderer(type: unknown): unknown {
    let current = type
    for (let depth = 0; depth < 5 && current && typeof current === "object"; depth += 1) {
        const wrapped = current as Wrapped
        current = wrapped.type ?? wrapped.render
    }
    return current
}

/** Display names and function names of a component and of the components it wraps. */
function names(type: unknown): string[] {
    const found: string[] = []
    let current = type
    for (let depth = 0; depth < 6 && current; depth += 1) {
        const {displayName, name} = current as Wrapped
        if (displayName) found.push(displayName)
        if (typeof current === "function" && name) found.push(name)
        current =
            typeof current === "object"
                ? ((current as Wrapped).type ?? (current as Wrapped).render)
                : undefined
    }
    return found
}

function root(container: Element): Fiber | undefined {
    const key = Object.keys(container).find((name) => name.startsWith("__reactContainer$"))
    const hostRoot = key
        ? (container as unknown as Record<string, {stateNode?: {current?: Fiber}}>)[key]
        : undefined
    return hostRoot?.stateNode?.current
}

/** The fiber, its siblings and all their descendants. */
function walk(start: Fiber | null): Fiber[] {
    const fibers: Fiber[] = []
    const stack: Fiber[] = start ? [start] : []
    while (stack.length) {
        const fiber = stack.pop() as Fiber
        fibers.push(fiber)
        if (fiber.sibling) stack.push(fiber.sibling)
        if (fiber.child) stack.push(fiber.child)
    }
    return fibers
}

const isInstance = (fiber: Fiber, component: unknown) =>
    fiber.elementType === component ||
    fiber.type === component ||
    (renderer(component) !== undefined && renderer(fiber.elementType) === renderer(component))

/** What a story has rendered so far. */
export interface WidgetObservation {
    /** Whether the section's component has been mounted. */
    mounted: boolean
    /** Names of the components mounted inside it. */
    shown: Set<string>
}

function scan(container: Element, component: unknown, observation: WidgetObservation) {
    const tree = root(container)
    if (!tree) return
    for (const instance of walk(tree).filter((fiber) => isInstance(fiber, component))) {
        observation.mounted = true
        for (const fiber of walk(instance.child)) {
            for (const name of names(fiber.elementType)) observation.shown.add(name)
        }
    }
}

/**
 * Starts recording what the story under `container` renders: now, after each
 * change of the document, and whenever `record` is called. Dialogs and tabs
 * that a story opens and closes again therefore count as rendered.
 */
export function observeRenderedWidgets(container: Element, component: unknown) {
    const observation: WidgetObservation = {mounted: false, shown: new Set()}
    const record = () => scan(container, component, observation)
    record()
    const observer = new MutationObserver(record)
    observer.observe(container.ownerDocument.body, {
        childList: true,
        subtree: true,
    })
    return {observation, record, stop: () => observer.disconnect()}
}

/**
 * Throws unless the story rendered `component`, and every name in `widgets`
 * inside one of its instances.
 */
export function assertRenderedWidgets(
    observation: WidgetObservation | undefined,
    component: unknown,
    widgets: string[] = []
) {
    if (!component) return
    const label = names(component)[0] ?? "the story component"
    if (!observation?.mounted) {
        throw new Error(`The story does not render ${label}, the component of its section`)
    }
    const missing = widgets.filter((name) => !observation.shown.has(name))
    if (missing.length) {
        throw new Error(
            `The story names ${missing.join(", ")} in parameters.widgets, but ${label} ` +
                `never rendered ${missing.length > 1 ? "them" : "it"}`
        )
    }
}
