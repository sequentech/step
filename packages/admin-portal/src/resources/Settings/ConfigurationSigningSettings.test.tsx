/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {StrictMode} from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ConfigurationSigningSettings} from "./ConfigurationSigningSettings"

const mockSet = jest.fn()
const mockRefetch = jest.fn()
const mockGetOne = jest.fn()
const mockQuery = jest.fn()

jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useMutation: () => [mockSet],
    useApolloClient: () => ({query: mockQuery}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("react-admin", () => ({
    useGetOne: (...args: unknown[]) => mockGetOne(...args),
}))

const ROOT = "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n"
const tenantA = {
    id: "tenant-a",
    settings: {
        configuration_signing: {
            policy: "required",
            package_roots: ROOT,
            staff_roots: ROOT,
            required_approvals: 2,
        },
    },
}

/** A tenant record as useGetOne returns it, fetched at `fetchedAt`. */
const loaded = (record: unknown, fetchedAt = 1) => ({
    data: record,
    refetch: mockRefetch,
    dataUpdatedAt: fetchedAt,
})

const save = () => fireEvent.click(screen.getByTestId("configuration-signing-save"))

describe("ConfigurationSigningSettings", () => {
    beforeEach(() => {
        jest.resetAllMocks()
        mockRefetch.mockResolvedValue({})
        // The server's stored setting, read back after a save.
        mockQuery.mockResolvedValue({
            data: {sequent_backend_tenant_by_pk: {id: "tenant-a", settings: tenantA.settings}},
        })
    })

    it("starts from the installed setting and saves it through the action", async () => {
        mockGetOne.mockReturnValue(loaded(tenantA))
        mockSet.mockResolvedValue({
            data: {
                set_configuration_signing: {
                    saved: true,
                    check: {anchors: [], report: {problems: []}},
                },
            },
        })
        render(<ConfigurationSigningSettings tenantId="tenant-a" />)
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.required"
        )
        expect(screen.getByTestId("configuration-signing-packageRoots")).toHaveValue(ROOT)
        save()
        await waitFor(() =>
            expect(screen.getByTestId("configuration-signing-result")).toBeInTheDocument()
        )
        expect(mockSet).toHaveBeenCalledWith({
            variables: {
                tenantId: "tenant-a",
                configurationSigning: {
                    policy: "required",
                    package_roots: ROOT,
                    staff_roots: ROOT,
                    required_approvals: 2,
                },
            },
        })
        expect(screen.getByText("configurationSigning.saved")).toBeInTheDocument()
        // The answer stays, and the state shows what was saved, without a
        // refetch that would re-render (and remount) the Settings tabs.
        expect(mockRefetch).not.toHaveBeenCalled()
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.required"
        )
    })

    it("still saves and shows the answer under StrictMode", async () => {
        mockGetOne.mockReturnValue(loaded(tenantA))
        mockSet.mockResolvedValue({
            data: {
                set_configuration_signing: {
                    saved: false,
                    check: {
                        anchors: [],
                        report: {
                            problems: [
                                {severity: "error", path: "package_roots", message: "expired"},
                            ],
                        },
                    },
                },
            },
        })
        render(
            <StrictMode>
                <ConfigurationSigningSettings tenantId="tenant-a" />
            </StrictMode>
        )
        save()
        await waitFor(() =>
            expect(screen.getByTestId("configuration-signing-problem")).toHaveTextContent(
                "package_roots: expired"
            )
        )
        expect(screen.getByText("configurationSigning.refused")).toBeInTheDocument()
        expect(mockRefetch).not.toHaveBeenCalled()
    })

    it("can't save before its own tenant's record has loaded", () => {
        // Still loading, or a record cached from another tenant.
        for (const record of [undefined, tenantA]) {
            mockGetOne.mockReturnValue(loaded(record))
            const {unmount} = render(<ConfigurationSigningSettings tenantId="tenant-b" />)
            expect(screen.getByTestId("configuration-signing-save")).toBeDisabled()
            expect(screen.getByTestId("configuration-signing-packageRoots")).toBeDisabled()
            expect(screen.getByTestId("configuration-signing-packageRoots")).toHaveValue("")
            save()
            expect(mockSet).not.toHaveBeenCalled()
            unmount()
        }
    })

    it("shows the stored setting after a save, where nothing was set before", async () => {
        mockGetOne.mockReturnValue(loaded({id: "tenant-b", settings: {}}))
        mockSet.mockResolvedValue({
            data: {
                set_configuration_signing: {
                    saved: true,
                    check: {anchors: [], report: {problems: []}},
                },
            },
        })
        render(<ConfigurationSigningSettings tenantId="tenant-b" />)
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.not-configured"
        )
        fireEvent.change(screen.getByTestId("configuration-signing-packageRoots"), {
            target: {value: ROOT},
        })
        fireEvent.change(screen.getByTestId("configuration-signing-staffRoots"), {
            target: {value: ROOT},
        })
        save()
        await waitFor(() =>
            expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
                "configurationSigning.states.required"
            )
        )
        expect(screen.getByTestId("configuration-signing-result")).toBeInTheDocument()
    })

    it("shows the stored setting the server reads back, not the submitted one", async () => {
        mockGetOne.mockReturnValue(loaded({id: "tenant-b", settings: {}}))
        mockSet.mockResolvedValue({
            data: {
                set_configuration_signing: {
                    saved: true,
                    check: {anchors: [], report: {problems: []}},
                },
            },
        })
        // The form submits Required; the server says what it stored.
        mockQuery.mockResolvedValue({
            data: {
                sequent_backend_tenant_by_pk: {
                    id: "tenant-b",
                    settings: {configuration_signing: {policy: "optional", required_approvals: 0}},
                },
            },
        })
        render(<ConfigurationSigningSettings tenantId="tenant-b" />)
        save()
        await waitFor(() =>
            expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
                "configurationSigning.states.optional"
            )
        )
        expect(mockQuery).toHaveBeenCalledWith(
            expect.objectContaining({
                variables: {tenantId: "tenant-b"},
                fetchPolicy: "network-only",
            })
        )
    })

    it("follows the tenant record again once it changes after a save", async () => {
        mockGetOne.mockReturnValue(loaded(tenantA))
        mockSet.mockResolvedValue({
            data: {
                set_configuration_signing: {
                    saved: true,
                    check: {anchors: [], report: {problems: []}},
                },
            },
        })
        const {rerender} = render(<ConfigurationSigningSettings tenantId="tenant-a" />)
        save()
        await waitFor(() =>
            expect(screen.getByTestId("configuration-signing-result")).toBeInTheDocument()
        )
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.required"
        )
        // Someone else changes the policy; the record is fetched again.
        mockGetOne.mockReturnValue(
            loaded(
                {
                    id: "tenant-a",
                    settings: {configuration_signing: {policy: "optional", required_approvals: 0}},
                },
                2
            )
        )
        rerender(<ConfigurationSigningSettings tenantId="tenant-a" />)
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.optional"
        )
        // The answer to this person's save stays on screen.
        expect(screen.getByTestId("configuration-signing-result")).toBeInTheDocument()
    })

    it("follows a later fetch even when it holds exactly the pre-save setting", async () => {
        // Before: optional. This person saves; the server stores required.
        const before = {
            id: "tenant-a",
            settings: {configuration_signing: {policy: "optional", required_approvals: 0}},
        }
        mockGetOne.mockReturnValue(loaded(before, 1))
        mockSet.mockResolvedValue({
            data: {
                set_configuration_signing: {
                    saved: true,
                    check: {anchors: [], report: {problems: []}},
                },
            },
        })
        const {rerender} = render(<ConfigurationSigningSettings tenantId="tenant-a" />)
        save()
        await waitFor(() =>
            expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
                "configurationSigning.states.required"
            )
        )
        // Same content as before the save, but fetched later: someone reverted it.
        rerender(<ConfigurationSigningSettings tenantId="tenant-a" />)
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.required"
        )
        mockGetOne.mockReturnValue(loaded(JSON.parse(JSON.stringify(before)), 3))
        rerender(<ConfigurationSigningSettings tenantId="tenant-a" />)
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.optional"
        )
    })

    it("keeps the read-back over a refresh of the old record that landed while saving", async () => {
        // Before: optional. The save is slow; meanwhile the record refreshes
        // (still optional, fetched at 2). The server then stores required.
        const before = {
            id: "tenant-a",
            settings: {configuration_signing: {policy: "optional", required_approvals: 0}},
        }
        mockGetOne.mockReturnValue(loaded(before, 1))
        let answer: (value: unknown) => void = () => undefined
        mockSet.mockReturnValue(new Promise((resolve) => (answer = resolve)))
        const {rerender} = render(<ConfigurationSigningSettings tenantId="tenant-a" />)
        save()
        mockGetOne.mockReturnValue(loaded(JSON.parse(JSON.stringify(before)), 2))
        rerender(<ConfigurationSigningSettings tenantId="tenant-a" />)
        await act(async () => {
            answer({
                data: {
                    set_configuration_signing: {
                        saved: true,
                        check: {anchors: [], report: {problems: []}},
                    },
                },
            })
        })
        await waitFor(() =>
            expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
                "configurationSigning.states.required"
            )
        )
        // Only a fetch after the read-back wins.
        mockGetOne.mockReturnValue(loaded(JSON.parse(JSON.stringify(before)), 3))
        rerender(<ConfigurationSigningSettings tenantId="tenant-a" />)
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.optional"
        )
    })

    it("offers required signatures with two approvals where none is set", () => {
        mockGetOne.mockReturnValue(loaded({id: "tenant-b", settings: {}}))
        render(<ConfigurationSigningSettings tenantId="tenant-b" />)
        expect(screen.getByTestId("configuration-signing-state")).toHaveTextContent(
            "configurationSigning.states.not-configured"
        )
        expect(screen.getByTestId("configuration-signing-approvals")).toHaveValue(2)
        expect(screen.getByTestId("configuration-signing-save")).toBeEnabled()
    })

    it("ignores an answer that arrives after it is gone", async () => {
        mockGetOne.mockReturnValue(loaded(tenantA))
        let answer: (value: unknown) => void = () => undefined
        mockSet.mockReturnValue(new Promise((resolve) => (answer = resolve)))
        const errors = jest.spyOn(console, "error").mockImplementation(() => undefined)
        const {unmount} = render(<ConfigurationSigningSettings tenantId="tenant-a" />)
        save()
        unmount()
        await act(async () => {
            answer({data: {set_configuration_signing: {saved: true, check: {}}}})
        })
        expect(mockRefetch).not.toHaveBeenCalled()
        expect(errors).not.toHaveBeenCalled()
        errors.mockRestore()
    })

    it("refuses an approval count out of range before sending", () => {
        mockGetOne.mockReturnValue(loaded(tenantA))
        render(<ConfigurationSigningSettings tenantId="tenant-a" />)
        fireEvent.change(screen.getByTestId("configuration-signing-approvals"), {
            target: {value: "17"},
        })
        save()
        expect(screen.getByTestId("configuration-signing-error")).toHaveTextContent(
            "configurationSigning.invalidApprovals"
        )
        expect(mockSet).not.toHaveBeenCalled()
    })

    describe("format 2", () => {
        const SIGNERS = [
            {
                id: "approver-1",
                name: "DEMO approver-1",
                roles: ["approver"],
                keys: [{spki_sha256: "a".repeat(64)}],
            },
            {
                id: "approver-2",
                name: "DEMO approver-2",
                roles: ["approver"],
                keys: [{spki_sha256: "b".repeat(64)}],
            },
            {
                id: "custodian",
                name: "DEMO custodian",
                roles: ["final-signer"],
                keys: [{spki_sha256: "c".repeat(64)}],
            },
        ]
        const policyFile = (tenant: string) =>
            new File(
                [
                    JSON.stringify({
                        tenant_id: tenant,
                        signing: {
                            policy: "required",
                            package_roots: ROOT,
                            staff_roots: ROOT,
                            required_approvals: 2,
                            minimum_package_format: "v2",
                            final_signer_separation: "separate",
                            signers: SIGNERS,
                        },
                    }),
                ],
                "signing-policy.json"
            )
        const upload = async (file: File) => {
            await act(async () => {
                fireEvent.change(screen.getByTestId("configuration-signing-policy-file"), {
                    target: {files: [file]},
                })
            })
        }

        it("loads this tenant's policy file, shows the signers and saves them with the digest shown", async () => {
            mockGetOne.mockReturnValue(loaded(tenantA))
            mockSet.mockResolvedValue({
                data: {
                    set_configuration_signing: {
                        saved: true,
                        check: {anchors: [], report: {problems: []}, policy_sha256: "d".repeat(64)},
                    },
                },
            })
            render(<ConfigurationSigningSettings tenantId="tenant-a" />)
            await upload(policyFile("tenant-a"))
            await waitFor(() =>
                expect(screen.getByTestId("configuration-signing-signers")).toBeInTheDocument()
            )
            expect(screen.getByTestId("configuration-signing-signer-custodian")).toHaveTextContent(
                "final-signer"
            )
            save()
            await waitFor(() =>
                expect(
                    screen.getByTestId("configuration-signing-policy-digest")
                ).toBeInTheDocument()
            )
            expect(screen.getByTestId("configuration-signing-policy-digest")).toHaveTextContent(
                "d".repeat(64)
            )
            const sent = mockSet.mock.calls[0][0].variables.configurationSigning
            expect(sent.minimum_package_format).toBe("v2")
            expect(sent.final_signer_separation).toBe("separate")
            expect(sent.signers.map((s: {id: string}) => s.id)).toEqual([
                "approver-1",
                "approver-2",
                "custodian",
            ])
        })

        it("refuses another tenant's policy file and keeps what was there", async () => {
            mockGetOne.mockReturnValue(loaded(tenantA))
            render(<ConfigurationSigningSettings tenantId="tenant-a" />)
            await upload(policyFile("tenant-b"))
            expect(await screen.findByTestId("configuration-signing-error")).toHaveTextContent(
                "configurationSigning.policyErrors.other-tenant"
            )
            expect(screen.queryByTestId("configuration-signing-signers")).not.toBeInTheDocument()
            expect(screen.getByTestId("configuration-signing-packageRoots")).toHaveValue(ROOT)
        })

        it("keeps a loaded valid policy when a later file is malformed", async () => {
            mockGetOne.mockReturnValue(loaded(tenantA))
            render(<ConfigurationSigningSettings tenantId="tenant-a" />)
            await upload(policyFile("tenant-a"))
            await waitFor(() =>
                expect(screen.getByTestId("configuration-signing-signers")).toBeInTheDocument()
            )
            const misspelled = new File(
                [
                    JSON.stringify({
                        tenant_id: "tenant-a",
                        signing: {policy: "required", minimum_package_format: "v2", signerz: []},
                    }),
                ],
                "signing-policy.json"
            )
            await upload(misspelled)
            expect(await screen.findByTestId("configuration-signing-error")).toHaveTextContent(
                "configurationSigning.policyErrors.unknown-field"
            )
            expect(screen.getByTestId("configuration-signing-signer-custodian")).toBeInTheDocument()
            await upload(
                new File([JSON.stringify({signing: {policy: "required"}})], "signing-policy.json")
            )
            // The error already shown changes in place: wait for the new one.
            await waitFor(() =>
                expect(screen.getByTestId("configuration-signing-error")).toHaveTextContent(
                    "configurationSigning.policyErrors.no-tenant"
                )
            )
            expect(screen.getByTestId("configuration-signing-signer-custodian")).toBeInTheDocument()
        })

        it("shows a stored format-2 record the importer refuses as unreadable", async () => {
            mockGetOne.mockReturnValue(
                loaded({
                    ...tenantA,
                    settings: {
                        configuration_signing: {
                            policy: "required",
                            required_approvals: 2,
                            signers: SIGNERS,
                        },
                    },
                })
            )
            render(<ConfigurationSigningSettings tenantId="tenant-a" />)
            expect(await screen.findByTestId("configuration-signing-state")).toHaveTextContent(
                "configurationSigning.states.unreadable"
            )
        })
    })
})
