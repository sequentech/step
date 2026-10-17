/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {StrictMode} from "react"
import {act, createEvent, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {RouterProvider, createMemoryRouter, useNavigate} from "react-router-dom"
import Projects from "./Projects"
import {PluginErrorCode, PluginHost, PluginRequestError} from "../contracts"
import {HistoryEvent, Operation, Permission, Saved, Task} from "./model"

// React Router needs the encoders and fetch Request that jsdom lacks.
jest.mock("react-router-dom", () => {
    const {TextEncoder, TextDecoder} = jest.requireActual("node:util")
    const {Request} = jest.requireActual("cross-fetch")
    Object.assign(globalThis, {TextEncoder, TextDecoder, Request})
    return jest.requireActual("react-router-dom")
})
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))
jest.mock("react-admin", () => {
    const router = jest.requireActual("react-router-dom")
    const React = jest.requireActual("react")
    return {
        useBlocker: router.useBlocker,
        useCanBlock: () => Boolean(React.useContext(router.UNSAFE_DataRouterContext)),
    }
})

const TZ = "Asia/Manila"
const project = {
    id: "p1",
    version: 1,
    updated_at: "2026-10-17T02:00:00Z",
    updated_by: "admin",
    data: {name: "Launch", timezone: TZ, state: "active"},
}
const taskData = (title: string, day: number, order: number): Task =>
    ({
        title,
        type: "task",
        start: `2026-10-${day}T08:00:00+08:00`,
        end: `2026-10-${day + 1}T08:00:00+08:00`,
        status: "not_started",
        notes: "",
        order,
    }) as Task

/** An in-memory Projects API with versions and conflicts. */
class FakeServer {
    tasks: Saved<Task>[] = [
        {
            id: "t1",
            version: 1,
            updated_at: "2026-10-17T02:00:00Z",
            updated_by: "admin",
            data: taskData("Prepare", 17, 0),
        },
        {
            id: "t2",
            version: 1,
            updated_at: "2026-10-17T02:00:00Z",
            updated_by: "admin",
            data: taskData("Review", 20, 1),
        },
    ]
    calls: Array<[string, Record<string, unknown>]> = []
    /** Operations whose next response is held until released. */
    held = new Map<string, Array<() => void>>()
    holdNext = new Set<string>()
    history: Record<string, HistoryEvent[]> = {}
    conflictNext = false

    execute = jest.fn(
        async (operation: string, data: Record<string, unknown>): Promise<unknown> => {
            this.calls.push([operation, data])
            if (this.holdNext.has(operation)) {
                this.holdNext.delete(operation)
                await new Promise<void>((resolve) =>
                    this.held.set(operation, [...(this.held.get(operation) ?? []), resolve])
                )
            }
            return this.respond(operation, data)
        }
    )

    release(operation: string) {
        const waiting = this.held.get(operation) ?? []
        this.held.delete(operation)
        waiting.forEach((resolve) => resolve())
    }

    respond(operation: string, data: Record<string, unknown>): unknown {
        switch (operation) {
            case Operation.ListProjects:
                return {
                    items: [
                        {
                            ...project,
                            task_count: this.tasks.length,
                            start: this.tasks[0]?.data.start,
                            end: null,
                        },
                    ],
                }
            case Operation.GetProject:
                return {project, tasks: this.tasks}
            case Operation.History:
                return {events: this.history[String(data.task_id ?? "project")] ?? []}
            case Operation.SaveTask: {
                const existing = this.tasks.find((task) => task.id === data.id)
                if (existing && (this.conflictNext || existing.version !== data.expected_version)) {
                    this.conflictNext = false
                    const current = {
                        ...existing,
                        version: existing.version + 1,
                        updated_by: "planner-b",
                        data: {...existing.data, title: "Their version"},
                    }
                    this.tasks = this.tasks.map((task) =>
                        task.id === existing.id ? current : task
                    )
                    throw new PluginRequestError(
                        "Changed elsewhere",
                        PluginErrorCode.Conflict,
                        current
                    )
                }
                const saved = {
                    id: existing?.id ?? `t${this.tasks.length + 1}`,
                    version: (existing?.version ?? 0) + 1,
                    updated_at: "2026-10-17T03:00:00Z",
                    updated_by: "admin",
                    data: data.task as Task,
                }
                this.tasks = existing
                    ? this.tasks.map((task) => (task.id === saved.id ? saved : task))
                    : [...this.tasks, saved]
                return saved
            }
            case Operation.DeleteTask:
                this.tasks = this.tasks.filter((task) => task.id !== data.id)
                return {deleted: data.id}
            case Operation.Export:
                return {csv: "id,title,type,start,end,status,notes\r\n", filename: "launch.csv"}
            case Operation.PreviewImport:
                return {
                    valid: false,
                    rows: [{line: 2, task: null, errors: ["type must be task or milestone"]}],
                }
            default:
                throw new PluginRequestError("Unknown", PluginErrorCode.NotFound)
        }
    }

    count(operation: string) {
        return this.calls.filter(([name]) => name === operation).length
    }
}

const permissions = {
    viewer: [Permission.Read],
    editor: [Permission.Read, Permission.Write],
    manager: [Permission.Read, Permission.Write, Permission.Manage],
}

const Menu = () => {
    const navigate = useNavigate()
    return (
        <button type="button" onClick={() => navigate("/settings")}>
            Settings menu
        </button>
    )
}

const setup = (roles: string[] = permissions.manager) => {
    const server = new FakeServer()
    const host: PluginHost = {
        execute: <T,>(operation: string, data: Record<string, unknown>) =>
            server.execute(operation, data) as Promise<T>,
        can: (permission) => roles.includes(permission),
    }
    const router = createMemoryRouter(
        [
            {
                path: "/plugins/projects",
                element: (
                    <>
                        <Menu />
                        <Projects host={host} />
                    </>
                ),
            },
            {path: "/settings", element: <p>Settings screen</p>},
        ],
        {initialEntries: ["/home", "/plugins/projects"], initialIndex: 1}
    )
    render(
        <StrictMode>
            <RouterProvider router={router} />
        </StrictMode>
    )
    return {server, router}
}

/** jsdom has no PointerEvent; set the pointer properties on the event itself. */
const pointer = (
    type: "pointerDown" | "pointerMove" | "pointerUp",
    element: HTMLElement,
    properties: Record<string, number | string>
) => {
    const event = createEvent[type](element)
    Object.entries(properties).forEach(([key, value]) => Object.defineProperty(event, key, {value}))
    fireEvent(element, event)
}
const bar = (title: RegExp) => screen.getByRole("button", {name: title})
const open = async () => {
    fireEvent.click(await screen.findByRole("button", {name: "Launch"}))
    await screen.findByRole("button", {name: /Prepare/})
}
const select = async (title: RegExp) => {
    fireEvent.click(bar(title))
    await screen.findByLabelText(/projects.editor.title/)
}
const titleField = () => screen.getByLabelText(/projects.editor.title/) as HTMLInputElement
const type = (value: string) => fireEvent.change(titleField(), {target: {value}})

beforeAll(() => {
    URL.createObjectURL = jest.fn(() => "blob:csv")
    URL.revokeObjectURL = jest.fn()
})

test("a viewer reads every task without any way to change it", async () => {
    const {server} = setup(permissions.viewer)
    await open()
    expect(screen.queryByRole("button", {name: /projects.detail.addTask/})).toBeNull()
    expect(screen.getByText(/projects.detail.viewOnly/)).toBeTruthy()
    expect(screen.getByText(/Asia\/Manila/)).toBeTruthy()
    await select(/Prepare/)
    expect(titleField().readOnly).toBe(true)
    expect(screen.queryByRole("button", {name: "projects.editor.save"})).toBeNull()
    fireEvent.keyDown(bar(/Review/), {key: "ArrowRight"})
    expect(server.count(Operation.SaveTask)).toBe(0)
    fireEvent.click(screen.getByRole("button", {name: /projects.detail.export/}))
    await waitFor(() => expect(server.count(Operation.Export)).toBe(1))
})

test("hover and focus show the same short preview; Escape dismisses it", async () => {
    setup()
    await open()
    fireEvent.pointerEnter(bar(/Prepare/), {pointerType: "mouse"})
    const preview = await screen.findByTestId("projects-task-preview")
    expect(within(preview).getByText("Prepare")).toBeTruthy()
    expect(within(preview).getByText(/2026-10-18 08:00/)).toBeTruthy()
    fireEvent.pointerLeave(bar(/Prepare/))
    await waitFor(() => expect(screen.queryByTestId("projects-task-preview")).toBeNull())
    act(() => bar(/Review/).focus())
    expect(
        within(await screen.findByTestId("projects-task-preview")).getByText("Review")
    ).toBeTruthy()
    fireEvent.keyDown(bar(/Review/), {key: "Escape"})
    await waitFor(() => expect(screen.queryByTestId("projects-task-preview")).toBeNull())
    // jsdom has no PointerEvent, so set the pointer type on the event itself.
    const touch = createEvent.pointerOver(bar(/Prepare/))
    Object.defineProperty(touch, "pointerType", {value: "touch"})
    fireEvent(bar(/Prepare/), touch)
    expect(screen.queryByTestId("projects-task-preview")).toBeNull()
})

test("a 30-minute task's preview says 30 minutes", async () => {
    const {server} = setup()
    server.tasks[0].data.end = "2026-10-17T08:30:00+08:00"
    await open()
    act(() => bar(/Prepare/).focus())
    const preview = await screen.findByTestId("projects-task-preview")
    expect(within(preview).getByText(/projects.units.minutes {"count":30}/)).toBeTruthy()
    expect(within(preview).queryByText(/projects.units.hours/)).toBeNull()
})

test("arrow keys move focus between tasks", async () => {
    setup()
    await open()
    bar(/Prepare/).focus()
    fireEvent.keyDown(bar(/Prepare/), {key: "ArrowDown"})
    expect(document.activeElement).toBe(bar(/Review/))
    fireEvent.keyDown(bar(/Review/), {key: "ArrowUp"})
    expect(document.activeElement).toBe(bar(/Prepare/))
})

test("moving the open task keeps unsaved text; a conflict needs an explicit reload", async () => {
    const {server} = setup()
    await open()
    await select(/Prepare/)
    type("My draft")
    fireEvent.keyDown(bar(/Prepare/), {key: "ArrowRight"})
    fireEvent.keyDown(bar(/Prepare/), {key: "ArrowRight"})
    fireEvent.keyDown(bar(/Prepare/), {key: "ArrowLeft"})
    expect(titleField().value).toBe("My draft")
    expect(server.count(Operation.SaveTask)).toBe(0)
    server.conflictNext = true
    fireEvent.click(screen.getByRole("button", {name: "projects.editor.save"}))
    await screen.findByText(/projects.conflict.body/)
    expect(titleField().value).toBe("My draft")
    expect(server.calls.find(([name]) => name === Operation.SaveTask)?.[1]).toMatchObject({
        expected_version: 1,
        task: {title: "My draft", start: "2026-10-18T08:00:00+08:00"},
    })
    fireEvent.click(screen.getByRole("button", {name: "projects.conflict.load"}))
    await waitFor(() => expect(titleField().value).toBe("Their version"))
})

test("dragging a bar saves a move of whole days", async () => {
    const {server} = setup()
    await open()
    const review = bar(/Review/)
    pointer("pointerDown", review, {button: 0, clientX: 100, pointerId: 1, pointerType: "mouse"})
    pointer("pointerMove", review, {clientX: 120, pointerId: 1, pointerType: "mouse"})
    pointer("pointerUp", review, {clientX: 120, pointerId: 1, pointerType: "mouse"})
    fireEvent.click(review)
    await waitFor(() => expect(server.count(Operation.SaveTask)).toBe(1))
    // The timeline is 2 px per day in jsdom, so 20 px is 10 days.
    expect(server.calls.find(([name]) => name === Operation.SaveTask)?.[1]).toMatchObject({
        id: "t2",
        task: {start: "2026-10-30T08:00:00+08:00", end: "2026-10-31T08:00:00+08:00"},
    })
    expect(screen.queryByLabelText(/projects.editor.title/)).toBeNull()
})

test("focusing the title right after selecting a task keeps the focus there", async () => {
    setup()
    await open()
    fireEvent.click(bar(/Prepare/))
    // The editor renders with the click; focus its title before any timer runs.
    const title = screen.getByLabelText(/projects.editor.title/)
    act(() => title.focus())
    await act(() => new Promise((resolve) => setTimeout(resolve, 20)))
    expect(document.activeElement).toBe(title)
})

test("unsaved changes are guarded when selecting, adding and closing", async () => {
    setup()
    await open()
    await select(/Prepare/)
    type("My draft")
    fireEvent.click(bar(/Review/))
    expect(await screen.findByText(/projects.unsaved.body/)).toBeTruthy()
    fireEvent.click(screen.getByRole("button", {name: "projects.unsaved.keep"}))
    expect(titleField().value).toBe("My draft")
    fireEvent.click(screen.getByRole("button", {name: /projects.detail.addTask/}))
    fireEvent.click(await screen.findByRole("button", {name: "projects.unsaved.discard"}))
    await waitFor(() => expect(titleField().value).toBe(""))
    expect(screen.getByText("projects.panel.add")).toBeTruthy()
})

test("a new task is guarded after any edit, even with a blank title", async () => {
    setup()
    await open()
    fireEvent.click(screen.getByRole("button", {name: /projects.detail.addTask/}))
    await screen.findByText("projects.panel.add")
    fireEvent.click(screen.getByRole("button", {name: "projects.panel.close"}))
    await waitFor(() => expect(screen.queryByText("projects.panel.add")).toBeNull())
    fireEvent.click(screen.getByRole("button", {name: /projects.detail.addTask/}))
    fireEvent.mouseDown(screen.getByLabelText(/projects.editor.status/))
    fireEvent.click(await screen.findByRole("option", {name: "projects.status.done"}))
    fireEvent.click(screen.getByRole("button", {name: "projects.panel.close"}))
    expect(await screen.findByText(/projects.unsaved.body/)).toBeTruthy()
})

test("menu navigation and browser Back wait for unsaved changes; Keep editing stays", async () => {
    const {router} = setup()
    await open()
    await select(/Prepare/)
    type("My draft")
    fireEvent.click(screen.getByRole("button", {name: "Settings menu"}))
    expect(await screen.findByText(/projects.unsaved.body/)).toBeTruthy()
    expect(router.state.location.pathname).toBe("/plugins/projects")
    fireEvent.click(screen.getByRole("button", {name: "projects.unsaved.keep"}))
    expect(titleField().value).toBe("My draft")
    expect(router.state.location.pathname).toBe("/plugins/projects")

    await act(async () => {
        await router.navigate(-1)
    })
    expect(await screen.findByText(/projects.unsaved.body/)).toBeTruthy()
    expect(router.state.location.pathname).toBe("/plugins/projects")
    fireEvent.click(screen.getByRole("button", {name: "projects.unsaved.keep"}))

    await act(async () => {
        await router.navigate(1)
    })
    expect(await screen.findByText(/projects.unsaved.body/)).toBeTruthy()
    fireEvent.click(screen.getByRole("button", {name: "projects.unsaved.keep"}))

    fireEvent.click(screen.getByRole("button", {name: "Settings menu"}))
    fireEvent.click(await screen.findByRole("button", {name: "projects.unsaved.discard"}))
    await screen.findByText("Settings screen")
    expect(router.state.location.pathname).toBe("/settings")
})

test("navigation is free once the changes are saved", async () => {
    const {router} = setup()
    await open()
    await select(/Prepare/)
    type("Saved title")
    fireEvent.click(screen.getByRole("button", {name: "projects.editor.save"}))
    await screen.findByText(/projects.notices.saved/)
    fireEvent.click(screen.getByRole("button", {name: "Settings menu"}))
    await screen.findByText("Settings screen")
    expect(router.state.location.pathname).toBe("/settings")
})

test("while a save is pending the panel cannot be replaced, and its response stays with it", async () => {
    const {server} = setup()
    await open()
    await select(/Prepare/)
    type("Pending title")
    server.holdNext.add(Operation.SaveTask)
    fireEvent.click(screen.getByRole("button", {name: "projects.editor.save"}))
    await waitFor(() => expect(server.held.get(Operation.SaveTask)?.length).toBe(1))
    fireEvent.click(bar(/Review/))
    expect(
        (screen.getByRole("button", {name: /projects.detail.addTask/}) as HTMLButtonElement)
            .disabled
    ).toBe(true)
    expect(
        (screen.getByRole("button", {name: "projects.panel.close"}) as HTMLButtonElement).disabled
    ).toBe(true)
    expect(screen.queryByText(/projects.unsaved.body/)).toBeNull()
    expect(titleField().value).toBe("Pending title")
    await act(async () => server.release(Operation.SaveTask))
    await screen.findByText(/projects.notices.saved/)
    expect(titleField().value).toBe("Pending title")
    expect(screen.getByText("Pending title", {selector: "p"})).toBeTruthy()
})

test("a task's next move waits for its previous save; another task's draft is kept", async () => {
    const {server} = setup()
    await open()
    await select(/Prepare/)
    type("Draft of Prepare")
    server.holdNext.add(Operation.SaveTask)
    fireEvent.keyDown(bar(/Review/), {key: "ArrowRight"})
    fireEvent.keyDown(bar(/Review/), {key: "ArrowRight"})
    await waitFor(() => expect(server.count(Operation.SaveTask)).toBe(1))
    expect(bar(/Review/).getAttribute("aria-busy")).toBe("true")
    await act(async () => server.release(Operation.SaveTask))
    await waitFor(() => expect(bar(/Review/).getAttribute("aria-busy")).toBeNull())
    fireEvent.keyDown(bar(/Review/), {key: "ArrowRight"})
    await waitFor(() => expect(server.count(Operation.SaveTask)).toBe(2))
    const saves = server.calls
        .filter(([name]) => name === Operation.SaveTask)
        .map(([, data]) => data)
    expect(saves.map((save) => save.expected_version)).toEqual([1, 2])
    expect(titleField().value).toBe("Draft of Prepare")
})

test("a delayed history response for a previous task is not shown", async () => {
    const {server} = setup(permissions.viewer)
    server.history.t1 = [
        {
            id: 1,
            record_id: "t1",
            parent_id: "p1",
            action: "update",
            actor: "first-task-editor",
            at: "2026-10-17T02:00:00Z",
            before: null,
            after: {data: {title: "Prepare"}},
        },
    ]
    server.history.t2 = [
        {
            id: 2,
            record_id: "t2",
            parent_id: "p1",
            action: "create",
            actor: "second-task-editor",
            at: "2026-10-17T02:00:00Z",
            before: null,
            after: {data: {title: "Review"}},
        },
    ]
    await open()
    server.holdNext.add(Operation.History)
    fireEvent.click(bar(/Prepare/))
    await waitFor(() => expect(server.held.get(Operation.History)?.length).toBe(1))
    fireEvent.click(bar(/Review/))
    expect(await screen.findByText(/second-task-editor/)).toBeTruthy()
    await act(async () => server.release(Operation.History))
    expect(screen.queryByText(/first-task-editor/)).toBeNull()
})

test("deleting asks first and keeps the project open", async () => {
    const {server} = setup()
    await open()
    await select(/Review/)
    fireEvent.click(screen.getByRole("button", {name: "projects.editor.delete"}))
    fireEvent.click(await screen.findByRole("button", {name: "projects.deleteDialog.confirm"}))
    await waitFor(() => expect(screen.queryByRole("button", {name: /Review/})).toBeNull())
    expect(server.calls.find(([name]) => name === Operation.DeleteTask)?.[1]).toMatchObject({
        id: "t2",
        expected_version: 1,
    })
})

test("an import with errors cannot create a project", async () => {
    const {server} = setup()
    fireEvent.click(await screen.findByRole("button", {name: "projects.list.import"}))
    fireEvent.change(screen.getByLabelText(/projects.create.name/), {target: {value: "Copy"}})
    const file = new File(["id,title\n"], "plan.csv", {type: "text/csv"})
    Object.defineProperty(file, "arrayBuffer", {
        value: async () => new TextEncoder().encode("id,title\n").buffer,
    })
    fireEvent.change(document.querySelector("input[type=file]")!, {target: {files: [file]}})
    expect(await screen.findByText("type must be task or milestone")).toBeTruthy()
    expect(
        (screen.getByRole("button", {name: "projects.import.commit"}) as HTMLButtonElement).disabled
    ).toBe(true)
    expect(server.count(Operation.CommitImport)).toBe(0)
})
