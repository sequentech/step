// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export {LocStudioApp} from "./App"
export {LocStudioProvider, useLocStudio, useCurrentScene} from "./LocStudioContext"
export {LocStudioEmbed} from "./embed/LocStudioEmbed"
export type {LocStudioEmbedProps} from "./embed/LocStudioEmbed"
export type {LocStudioSaveResult} from "./saveResult"
export {
    buildLocStudioSaveResult,
    mergePresentationI18n,
    presentationI18nFromSaveResult,
} from "./saveResult"
export {SCENES, getScene, getVariant, isVoteRouteScene, VOTE_ROUTE_SCENES} from "./catalog"
export type {SceneDefinition, SceneVariant, VoteRouteSceneId} from "./catalog"
export {
    parseUploadedElectionEvent,
    exportUploadedElectionEvent,
    isContentKey,
} from "./uploadedElection"
export type {UploadedElectionEvent} from "./uploadedElection"
export type {OverridesByLanguage} from "./i18n"
