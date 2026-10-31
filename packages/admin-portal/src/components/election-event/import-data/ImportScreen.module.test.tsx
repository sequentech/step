/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
jest.mock("./HistoricalImportPlugin", () => {
    throw new Error("The converter module must not be evaluated by the standard import screen")
})
jest.mock("@sequentech/ui-essentials", () => ({}), {virtual: true})
jest.mock("@apollo/client", () => ({gql: jest.fn()}))
jest.mock("react-admin", () => ({}))
jest.mock("@/components/styles/FormStyles", () => ({FormStyles: {PasswordInput: "input"}}))
jest.mock("@/providers/SettingsContextProvider", () => ({SettingsContext: {}}))
it("loads the standard import screen without evaluating the optional converter module", () => {
    expect(() => require("./ImportScreen")).not.toThrow()
})
