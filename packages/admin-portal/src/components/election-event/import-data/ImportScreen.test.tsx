/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ImportScreen} from "./ImportScreen"
import {EFileIntegrityPolicy} from "../../../../../ui-core/src/types/ElectionEventPresentation"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../../ui-core/src/types/ElectionEventPresentation")
)
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ImportRequestError} from "@/services/ImportRequestError"

jest.mock("./HistoricalImportPlugin", () => ({
    HistoricalImportPlugin: (props: {onAccepted: (file: File, hash: string) => Promise<void>}) => (
        <button
            onClick={() =>
                props.onAccepted(
                    new File(["synthetic ZIP"], "reviewed.zip", {type: "application/zip"}),
                    "ab".repeat(32)
                )
            }
        >
            Accept synthetic reviewed package
        </button>
    ),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL: "/wasm/converter"},
    }),
}))

const mockGetUploadUrl = jest.fn()
const mockNotify = jest.fn()
const mockFetch = jest.fn()

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [mockGetUploadUrl],
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    SimpleForm: ({children}: React.PropsWithChildren) => children,
}))
jest.mock("@/components/styles/FormStyles", () => ({
    FormStyles: {
        StatusBox: "div",
        ErrorMessage: "p",
        ShowProgress: () => require("react").createElement("div", {role: "progressbar"}),
        PasswordInput: ({
            inputProps,
            onChange,
        }: {
            inputProps: React.InputHTMLAttributes<HTMLInputElement>
            onChange: React.ChangeEventHandler<HTMLInputElement>
        }) => require("react").createElement("input", {...inputProps, type: "password", onChange}),
    },
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        // The shared list is tested in ui-essentials; here only which of the two
        // explanations the screen chooses.
        ProblemList: ({report}: {report: {problems: Array<{id?: string}>}}) =>
            require("react").createElement(
                "ul",
                {"aria-label": "Problems"},
                report.problems.map((problem, index) =>
                    require("react").createElement("li", {key: index}, problem.id)
                )
            ),
        DropFile: ({
            handleFiles,
            multiple,
            formatLabel,
        }: {
            handleFiles: (files: FileList | null) => Promise<void>
            multiple?: boolean
            formatLabel?: string
        }) =>
            require("react").createElement("input", {
                "aria-label": "Import file",
                "multiple": multiple,
                "data-format-label": formatLabel,
                "type": "file",
                "onChange": (event: React.ChangeEvent<HTMLInputElement>) =>
                    handleFiles(event.target.files),
            }),
        Dialog: ({
            open,
            title,
            ok,
            cancel,
            handleClose,
            children,
        }: React.PropsWithChildren<{
            open: boolean
            title: string
            ok: string
            cancel?: string
            handleClose: (confirmed: boolean) => void
        }>) =>
            open
                ? require("react").createElement(
                      "div",
                      {"role": "dialog", "aria-label": title},
                      children,
                      require("react").createElement(
                          "button",
                          {onClick: () => handleClose(true)},
                          ok
                      ),
                      cancel &&
                          require("react").createElement(
                              "button",
                              {onClick: () => handleClose(false)},
                              cancel
                          )
                  )
                : null,
    }),
    {virtual: true}
)

const UPLOAD_URL = "https://upload.invalid/event.json"
const DOCUMENT_ID = "11111111-1111-4111-8111-111111111111"
const CHECKSUM = "01".repeat(32)
const originalFetch = globalThis.fetch

beforeEach(() => {
    jest.clearAllMocks()
    mockGetUploadUrl.mockResolvedValue({
        data: {get_upload_url: {url: UPLOAD_URL, document_id: DOCUMENT_ID}},
    })
    mockFetch.mockResolvedValue({ok: true})
    globalThis.fetch = mockFetch
})
afterAll(() => {
    globalThis.fetch = originalFetch
})

function renderImport(allowHistoricalSource = false, fileIntegrityPolicy?: EFileIntegrityPolicy) {
    const uploadCallback = jest.fn(async () => {})
    const doImport = jest.fn(async () => {})
    const doCancel = jest.fn()
    render(
        <ImportScreen
            uploadCallback={uploadCallback}
            doImport={doImport}
            doCancel={doCancel}
            errors={null}
            fileIntegrityPolicy={fileIntegrityPolicy}
            allowHistoricalSource={allowHistoricalSource}
        />
    )
    if (allowHistoricalSource)
        fireEvent.change(screen.getByLabelText("Import file"), {
            target: {files: [new File(["sql"], "source.sql"), new File(["xlsx"], "source.xlsx")]},
        })
    return {uploadCallback, doImport, doCancel}
}

function uploadFile() {
    const file = new File(['{"elections":[]}'], "event.json", {type: "application/json"})
    fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [file]}})
    return file
}

it("uploads the private file before enabling import and forwards its integrity check", async () => {
    const callbacks = renderImport()
    const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    expect(importButton).toBeDisabled()
    fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
        target: {value: CHECKSUM},
    })
    let finishUpload: ((response: {ok: boolean}) => void) | undefined
    mockFetch.mockImplementation(
        () =>
            new Promise((resolve) => {
                finishUpload = resolve
            })
    )
    const file = uploadFile()
    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1))
    expect(importButton).toBeDisabled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(mockGetUploadUrl).toHaveBeenCalledWith({
        variables: {name: "event.json", media_type: "application/json", size: 16, is_public: false},
    })
    expect(mockFetch).toHaveBeenCalledWith(UPLOAD_URL, {
        method: "PUT",
        headers: {"Content-Type": "application/json"},
        body: file,
    })
    await act(async () => finishUpload?.({ok: true}))
    expect(callbacks.uploadCallback).toHaveBeenCalledWith(DOCUMENT_ID, "", CHECKSUM)
    expect(importButton).toBeEnabled()
    fireEvent.click(importButton)
    await waitFor(() => expect(callbacks.doImport).toHaveBeenCalledWith(DOCUMENT_ID, CHECKSUM, ""))
    expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadSuccess", {
        type: "success",
    })
})

it.each([
    ["HTTP 503", () => mockFetch.mockResolvedValue({ok: false, status: 503})],
    ["connection failure", () => mockFetch.mockRejectedValue(new TypeError("connection lost"))],
])(
    "keeps failed %s uploads unavailable for import and permits retry",
    async (_reason, injectFailure) => {
        const callbacks = renderImport()
        injectFailure()
        uploadFile()
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
                type: "error",
            })
        )
        expect(callbacks.uploadCallback).not.toHaveBeenCalled()
        expect(callbacks.doImport).not.toHaveBeenCalled()
        expect(
            screen.getByRole("button", {name: "electionEventScreen.import.import"})
        ).toBeDisabled()
        expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()
        mockFetch.mockResolvedValue({ok: true})
        uploadFile()
        await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
        expect(
            screen.getByRole("button", {name: "electionEventScreen.import.import"})
        ).toBeEnabled()
    }
)

it("releases the form when no upload URL is returned and allows a later upload", async () => {
    const callbacks = renderImport()
    mockGetUploadUrl.mockResolvedValueOnce({data: {get_upload_url: {document_id: DOCUMENT_ID}}})
    uploadFile()
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
            type: "error",
        })
    )
    expect(mockFetch).not.toHaveBeenCalled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()
    expect(screen.getByRole("button", {name: "electionEventScreen.import.cancel"})).toBeEnabled()
    expect(screen.getByRole("button", {name: "electionEventScreen.import.import"})).toBeDisabled()
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
})

it("reports an upload URL request failure without issuing a PUT", async () => {
    const callbacks = renderImport()
    mockGetUploadUrl.mockRejectedValue(new Error("service unavailable"))
    uploadFile()
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
            type: "error",
        })
    )
    expect(mockFetch).not.toHaveBeenCalled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()
})

it.each([
    [
        "missing URL",
        () =>
            mockGetUploadUrl.mockResolvedValueOnce({
                data: {get_upload_url: {document_id: "second-document"}},
            }),
    ],
    ["HTTP 503", () => mockFetch.mockResolvedValueOnce({ok: false, status: 503})],
    ["connection failure", () => mockFetch.mockRejectedValueOnce(new TypeError("connection lost"))],
    [
        "URL service failure",
        () => mockGetUploadUrl.mockRejectedValueOnce(new Error("service unavailable")),
    ],
])(
    "invalidates the previous uploaded document when a replacement fails with %s",
    async (_reason, injectFailure) => {
        const callbacks = renderImport()
        const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
        fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
            target: {value: CHECKSUM},
        })
        uploadFile()
        await waitFor(() =>
            expect(callbacks.uploadCallback).toHaveBeenCalledWith(DOCUMENT_ID, "", CHECKSUM)
        )
        expect(importButton).toBeEnabled()

        const replacementChecksum = "23".repeat(32)
        fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
            target: {value: replacementChecksum},
        })
        injectFailure()
        uploadFile()
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
                type: "error",
            })
        )
        expect(importButton).toBeDisabled()
        fireEvent.click(importButton)
        expect(callbacks.doImport).not.toHaveBeenCalled()
        expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1)
        expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()

        mockGetUploadUrl.mockResolvedValueOnce({
            data: {get_upload_url: {url: UPLOAD_URL, document_id: "replacement-document"}},
        })
        uploadFile()
        await waitFor(() =>
            expect(callbacks.uploadCallback).toHaveBeenCalledWith(
                "replacement-document",
                "",
                replacementChecksum
            )
        )
        fireEvent.click(importButton)
        await waitFor(() =>
            expect(callbacks.doImport).toHaveBeenCalledWith(
                "replacement-document",
                replacementChecksum,
                ""
            )
        )
        expect(callbacks.doImport).toHaveBeenCalledTimes(1)
    }
)

it("keeps import unavailable when the replacement upload callback fails", async () => {
    const callbacks = renderImport()
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
    const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    expect(importButton).toBeEnabled()
    callbacks.uploadCallback.mockRejectedValueOnce(new Error("invalid replacement document"))
    uploadFile()
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
            type: "error",
        })
    )
    expect(importButton).toBeDisabled()
    expect(callbacks.doImport).not.toHaveBeenCalled()
})

it("invalidates a previous document as soon as a replacement encrypted file is selected", async () => {
    const callbacks = renderImport()
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
    const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    expect(importButton).toBeEnabled()
    fireEvent.change(screen.getByLabelText("Import file"), {
        target: {files: [new File(["encrypted"], "event.ezip")]},
    })
    expect(screen.getByRole("dialog")).toBeInTheDocument()
    expect(importButton).toBeDisabled()
    expect(mockGetUploadUrl).toHaveBeenCalledTimes(1)
    expect(callbacks.doImport).not.toHaveBeenCalled()
})

it("shows the backend's error text when it named no problems", () => {
    render(
        <ImportScreen
            doImport={jest.fn()}
            doCancel={jest.fn()}
            errors="Error checking import: something broke"
            problems={null}
        />
    )
    expect(screen.getByText("Error checking import: something broke")).toBeInTheDocument()
    expect(screen.queryByRole("list", {name: "Problems"})).not.toBeInTheDocument()
})

it("names each problem instead of repeating the English paragraph", () => {
    render(
        <ImportScreen
            doImport={jest.fn()}
            doCancel={jest.fn()}
            errors="The election event bundle cannot be imported; 1 problem found:"
            problems={[
                {
                    severity: "error",
                    code: "unreadable",
                    path: "file",
                    message: "the file is not a readable election event",
                    id: "file.not-json",
                },
            ]}
        />
    )
    expect(screen.getByRole("list", {name: "Problems"})).toHaveTextContent("file.not-json")
    expect(screen.queryByText(/cannot be imported/)).not.toBeInTheDocument()
})

it("requires the certified hash in required mode without offering a bypass", async () => {
    const callbacks = renderImport(false, EFileIntegrityPolicy.REQUIRED_SHA256)
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
    const hash = screen.getByRole("textbox", {name: /electionEventScreen.import.sha/})
    const button = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    expect(hash).toBeRequired()
    expect(screen.getByText("electionEventScreen.import.shaRequiredHelp")).toBeInTheDocument()
    expect(button).toBeDisabled()
    fireEvent.change(hash, {target: {value: "not-a-sha256"}})
    expect(button).toBeDisabled()
    fireEvent.click(button)
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    expect(callbacks.doImport).not.toHaveBeenCalled()
    fireEvent.change(hash, {target: {value: "AB".repeat(32)}})
    expect(button).toBeEnabled()
    fireEvent.click(button)
    await waitFor(() =>
        expect(callbacks.doImport).toHaveBeenCalledWith(DOCUMENT_ID, "AB".repeat(32), "")
    )
})

it("preserves optional imports with the existing explicit bypass confirmation", async () => {
    const callbacks = renderImport()
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
    fireEvent.click(screen.getByRole("button", {name: "electionEventScreen.import.import"}))
    expect(callbacks.doImport).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole("button", {name: "electionEventScreen.import.shaDialog.ok"}))
    await waitFor(() => expect(callbacks.doImport).toHaveBeenCalledWith(DOCUMENT_ID, "", ""))
})

it("routes dropped SQL and XLSX to local conversion without uploading their raw contents", async () => {
    renderImport(true)
    const sql = new File(["synthetic SQL"], "source.SQL")
    const workbook = new File(["synthetic workbook"], "supplement.xlsx")
    fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [sql, workbook]}})
    await act(async () => {})
    expect(mockGetUploadUrl).not.toHaveBeenCalled()
    expect(mockFetch).not.toHaveBeenCalled()
    expect(screen.getByRole("button", {name: "electionEventScreen.import.import"})).toBeDisabled()
})

it("makes no backend request before explicit acceptance and separates dry-run from creation", async () => {
    const callbacks = renderImport(true)
    expect(mockGetUploadUrl).not.toHaveBeenCalled()
    expect(mockFetch).not.toHaveBeenCalled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(callbacks.doImport).not.toHaveBeenCalled()
    let finishDryRun: (() => void) | undefined
    callbacks.uploadCallback.mockImplementation(
        () =>
            new Promise<void>((resolve) => {
                finishDryRun = resolve
            })
    )
    fireEvent.click(await screen.findByRole("button", {name: "Accept synthetic reviewed package"}))
    await waitFor(() =>
        expect(screen.getByTestId("import-stage")).toHaveTextContent("stage.local-ready")
    )
    expect(mockGetUploadUrl).not.toHaveBeenCalled()
    expect(mockFetch).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole("button", {name: "electionEventScreen.import.create"}))
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
    expect(screen.getByTestId("import-stage")).toHaveTextContent("stage.validating")
    expect(screen.getByRole("button", {name: "electionEventScreen.import.create"})).toBeDisabled()
    expect(callbacks.doImport).not.toHaveBeenCalled()
    await act(async () => finishDryRun?.())
    await waitFor(() => expect(callbacks.doImport).toHaveBeenCalledTimes(1))
    expect(screen.getByTestId("import-stage")).toHaveTextContent("stage.submitted")
})
it.each(["transport", "validation"] as const)(
    "identifies backend dry-run %s failure and blocks creation",
    async (kind) => {
        const callbacks = renderImport(true)
        callbacks.uploadCallback.mockRejectedValue(new ImportRequestError(kind))
        fireEvent.click(
            await screen.findByRole("button", {name: "Accept synthetic reviewed package"})
        )
        await waitFor(() =>
            expect(screen.getByTestId("import-stage")).toHaveTextContent("stage.local-ready")
        )
        fireEvent.click(screen.getByRole("button", {name: "electionEventScreen.import.create"}))
        await waitFor(() =>
            expect(screen.getByTestId("import-stage")).toHaveTextContent(
                kind === "transport" ? "stage.validation-transport-error" : "stage.validation-error"
            )
        )
        expect(callbacks.doImport).not.toHaveBeenCalled()
        const create = screen.getByRole("button", {name: "electionEventScreen.import.create"})
        if (kind === "validation") expect(create).toBeDisabled()
        else expect(create).toBeEnabled()
    }
)
it("does not confuse a passing dry-run with successful creation and permits correction", async () => {
    const callbacks = renderImport(true)
    callbacks.doImport.mockRejectedValue(new ImportRequestError("creation"))
    fireEvent.click(await screen.findByRole("button", {name: "Accept synthetic reviewed package"}))
    await waitFor(() =>
        expect(screen.getByTestId("import-stage")).toHaveTextContent("stage.local-ready")
    )
    fireEvent.click(screen.getByRole("button", {name: "electionEventScreen.import.create"}))
    await waitFor(() =>
        expect(screen.getByTestId("import-stage")).toHaveTextContent("stage.creation-error")
    )
    expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeDisabled()
})

it("keeps native import working and blocks raw source upload when the plugin is unconfigured", async () => {
    const uploadCallback = jest.fn(async () => {})
    const doImport = jest.fn(async () => {})
    render(
        <SettingsContext.Provider
            value={{loaded: true, globalSettings: {}} as React.ContextType<typeof SettingsContext>}
        >
            <ImportScreen
                allowHistoricalSource
                uploadCallback={uploadCallback}
                doImport={doImport}
                doCancel={jest.fn()}
                errors={null}
            />
        </SettingsContext.Provider>
    )
    const sql = new File(["synthetic SQL"], "example.sql")
    fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [sql]}})
    expect(mockGetUploadUrl).not.toHaveBeenCalled()
    expect(mockFetch).not.toHaveBeenCalled()
    expect(uploadCallback).not.toHaveBeenCalled()
    expect(doImport).not.toHaveBeenCalled()
    fireEvent.change(screen.getByLabelText("Import file"), {
        target: {files: [new File(["zip"], "event.zip")]},
    })
    await waitFor(() => expect(uploadCallback).toHaveBeenCalledTimes(1))
    expect(screen.getByRole("button", {name: "electionEventScreen.import.import"})).toBeEnabled()
})

it("resets old backend diagnostics when a new source selection starts", async () => {
    const onSourceChange = jest.fn()
    render(
        <ImportScreen
            allowHistoricalSource
            onSourceChange={onSourceChange}
            doImport={jest.fn()}
            doCancel={jest.fn()}
            errors="Previous package refused"
        />
    )
    fireEvent.change(screen.getByLabelText("Import file"), {
        target: {files: [new File(["sql"], "replacement.sql")]},
    })
    await screen.findByRole("button", {name: "Accept synthetic reviewed package"})
    expect(onSourceChange).toHaveBeenCalledTimes(1)
    expect(mockGetUploadUrl).not.toHaveBeenCalled()
})

const downloadOptions = "electionEventScreen.import.downloadOptions"
const downloadZip = "electionEventScreen.import.downloadZip"

it("downloads the exact accepted file repeatedly without backend requests before Create", async () => {
    const createUrl = jest.fn((_file: Blob) => "blob:synthetic-download")
    Object.defineProperty(URL, "createObjectURL", {value: createUrl, configurable: true})
    Object.defineProperty(URL, "revokeObjectURL", {value: jest.fn(), configurable: true})
    const click = jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {})
    const callbacks = renderImport(true)
    expect(screen.queryByRole("button", {name: downloadOptions})).not.toBeInTheDocument()
    fireEvent.click(await screen.findByRole("button", {name: "Accept synthetic reviewed package"}))
    for (let attempt = 0; attempt < 2; attempt++) {
        fireEvent.click(await screen.findByRole("button", {name: downloadOptions}))
        fireEvent.click(await screen.findByRole("menuitem", {name: downloadZip}))
    }
    expect(createUrl).toHaveBeenCalledTimes(2)
    const downloaded = createUrl.mock.calls[0][0] as File
    expect(downloaded.name).toBe("reviewed.zip")
    expect(downloaded.type).toBe("application/zip")
    expect(createUrl.mock.calls[1][0]).toBe(downloaded)
    expect(mockFetch).not.toHaveBeenCalled()
    expect(mockGetUploadUrl).not.toHaveBeenCalled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(callbacks.doImport).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole("button", {name: "electionEventScreen.import.create"}))
    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1))
    expect(mockFetch.mock.calls[0][1].body).toBe(downloaded)
    await waitFor(() => expect(callbacks.doImport).toHaveBeenCalledTimes(1))
    click.mockRestore()
})

it("removes the old download when sources change or native import is selected", async () => {
    renderImport(true)
    fireEvent.click(await screen.findByRole("button", {name: "Accept synthetic reviewed package"}))
    await screen.findByRole("button", {name: downloadOptions})
    fireEvent.change(screen.getByLabelText("Import file"), {
        target: {files: [new File(["new"], "replacement.sql")]},
    })
    expect(screen.queryByRole("button", {name: downloadOptions})).not.toBeInTheDocument()
    fireEvent.click(await screen.findByRole("button", {name: "Accept synthetic reviewed package"}))
    await screen.findByRole("button", {name: downloadOptions})
    fireEvent.change(screen.getByLabelText("Import file"), {
        target: {files: [new File(["zip"], "native.zip", {type: "application/zip"})]},
    })
    expect(screen.queryByRole("button", {name: downloadOptions})).not.toBeInTheDocument()
    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1))
})

it.each([undefined, "", "   ", null, false])(
    "has standard native-only UI when the global setting is %s",
    async (setting) => {
        render(
            <SettingsContext.Provider
                value={
                    {
                        loaded: true,
                        globalSettings: {ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL: setting},
                    } as React.ContextType<typeof SettingsContext>
                }
            >
                <ImportScreen
                    allowHistoricalSource
                    doImport={jest.fn()}
                    doCancel={jest.fn()}
                    errors={null}
                />
            </SettingsContext.Provider>
        )
        expect(screen.getByLabelText("Import file")).not.toHaveAttribute("multiple")
        expect(screen.getByLabelText("Import file")).toHaveAttribute(
            "data-format-label",
            "ZIP / JSON / EZIP"
        )
        expect(screen.queryByRole("button", {name: downloadOptions})).not.toBeInTheDocument()
        fireEvent.change(screen.getByLabelText("Import file"), {
            target: {files: [new File(["sql"], "input.sql"), new File(["xlsx"], "input.xlsx")]},
        })
        expect(await screen.findByRole("alert")).toHaveTextContent("not configured")
        expect(
            screen.queryByRole("button", {name: "Accept synthetic reviewed package"})
        ).not.toBeInTheDocument()
        expect(mockFetch).not.toHaveBeenCalled()
        expect(mockGetUploadUrl).not.toHaveBeenCalled()
    }
)

it.each([undefined, "/wasm/replacement"])(
    "invalidates an accepted package if global configuration changes to %s",
    async (replacementUrl) => {
        const callbacks = {
            doImport: jest.fn(),
            doCancel: jest.fn(),
            errors: null,
            allowHistoricalSource: true,
        }
        const renderWith = (baseUrl?: string) => (
            <SettingsContext.Provider
                value={
                    {
                        loaded: true,
                        globalSettings: {ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL: baseUrl},
                    } as React.ContextType<typeof SettingsContext>
                }
            >
                <ImportScreen {...callbacks} />
            </SettingsContext.Provider>
        )
        const view = render(renderWith("/wasm/converter"))
        fireEvent.change(screen.getByLabelText("Import file"), {
            target: {files: [new File(["sql"], "input.sql"), new File(["xlsx"], "input.xlsx")]},
        })
        fireEvent.click(
            await screen.findByRole("button", {name: "Accept synthetic reviewed package"})
        )
        await screen.findByRole("button", {name: downloadOptions})
        view.rerender(renderWith(replacementUrl))
        expect(screen.queryByRole("button", {name: downloadOptions})).not.toBeInTheDocument()
        expect(
            screen.queryByRole("button", {name: "Accept synthetic reviewed package"})
        ).not.toBeInTheDocument()
        expect(
            screen.getByRole("button", {name: "electionEventScreen.import.import"})
        ).toBeDisabled()
        expect(mockFetch).not.toHaveBeenCalled()
    }
)

it("reports download failures safely and retries without consuming the accepted package", async () => {
    const createUrl = jest.fn((_file: Blob) => "blob:retry-download")
    Object.defineProperty(URL, "createObjectURL", {value: createUrl, configurable: true})
    const revokeUrl = jest.fn()
    Object.defineProperty(URL, "revokeObjectURL", {value: revokeUrl, configurable: true})
    const click = jest
        .spyOn(HTMLAnchorElement.prototype, "click")
        .mockImplementationOnce(() => {
            throw new Error("private browser detail")
        })
        .mockImplementation(() => {})
    renderImport(true)
    fireEvent.click(await screen.findByRole("button", {name: "Accept synthetic reviewed package"}))
    fireEvent.click(await screen.findByRole("button", {name: downloadOptions}))
    fireEvent.click(await screen.findByRole("menuitem", {name: downloadZip}))
    expect(await screen.findByRole("alert")).toHaveTextContent(
        "electionEventScreen.import.downloadError"
    )
    expect(screen.queryByText("private browser detail")).not.toBeInTheDocument()
    expect(document.querySelector("a[download]")).toBeNull()
    expect(screen.getByRole("button", {name: "electionEventScreen.import.create"})).toBeEnabled()
    expect(mockGetUploadUrl).not.toHaveBeenCalled()
    expect(mockFetch).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole("button", {name: downloadOptions}))
    fireEvent.click(await screen.findByRole("menuitem", {name: downloadZip}))
    expect(screen.queryByRole("alert")).not.toBeInTheDocument()
    expect(createUrl.mock.calls[1][0]).toBe(createUrl.mock.calls[0][0])
    await waitFor(() => expect(revokeUrl).toHaveBeenCalledWith("blob:retry-download"), {
        timeout: 2000,
    })
    expect(mockFetch).not.toHaveBeenCalled()
    click.mockRestore()
})

it("imports a reviewed historical package with its generated hash under the required policy", async () => {
    const callbacks = renderImport(true, EFileIntegrityPolicy.REQUIRED_SHA256)
    fireEvent.click(await screen.findByRole("button", {name: "Accept synthetic reviewed package"}))
    const create = await screen.findByRole("button", {name: "electionEventScreen.import.create"})
    expect(create).toBeEnabled()
    fireEvent.click(create)
    await waitFor(() =>
        expect(callbacks.doImport).toHaveBeenCalledWith(DOCUMENT_ID, "ab".repeat(32), "")
    )
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
})
