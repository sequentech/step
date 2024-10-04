// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { IMiruTransmissionPackageData } from "@/types/miru"
import React, { createContext, useContext, useRef, useState } from "react"

interface CustomNavigationContextProps {
	setFiltersRef: any
	displayFiltersRef: any
}

const defaultCustomNavigationContext: CustomNavigationContextProps = {
	setFiltersRef: null,
	displayFiltersRef: null
}

export const CustomNavigationContext = createContext<CustomNavigationContextProps>(
	defaultCustomNavigationContext
)

interface CustomNavigationContextProviderProps {
	children: JSX.Element
}

export const CustomNavigationContextProvider = (
	props: CustomNavigationContextProviderProps
) => {
	const setFiltersRef = useRef(null)
	const displayFiltersRef = useRef(null)

	return (
		<CustomNavigationContext.Provider
			value={{
				setFiltersRef,
				displayFiltersRef
			}}
		>
			{props.children}
		</CustomNavigationContext.Provider>
	)
}

export const useNavigationStore: () => {
	setFiltersRef: any
	displayFiltersRef: any
} = () => {
	const {
		setFiltersRef,
		displayFiltersRef
	} = useContext(CustomNavigationContext)
	return {
		setFiltersRef,
		displayFiltersRef
	}
}
