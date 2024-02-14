export enum EPublishStatus {
    Void = "VOID",
    Started = "STARTED",
    StartedLoading = "STARTED_LOADING",
    Paused = "PAUSED",
    PausedLoading = "PAUSED_LOADING",
    Stopped = "STOPPED",
    StoppedLoading = "STOPPED_LOADING",
    Published = "PUBLISHED",
    PublishedLoading = "PUBLISHED_LOADING",
    Generated = "GENERATED",
    GeneratedLoading = "GENERATED_LOADING",
}

export const nextStatus = (statusValue: EPublishStatus): EPublishStatus => {
    let statusIndex = Object.values(EPublishStatus).indexOf(statusValue)
    return Object.values(EPublishStatus)[statusIndex + 1]
}

export enum EPublishStatushChanges {
    Open = "OPEN",
    Paused = "PAUSED",
    Closed = "CLOSED",
    NotStarted = "NOT_STARTED",
}

export const PUBLISH_STATUS_CONVERT = {
    [EPublishStatushChanges.NotStarted]: EPublishStatus.Void,
    [EPublishStatushChanges.Open]: EPublishStatus.Started,
    [EPublishStatushChanges.Paused]: EPublishStatus.Paused,
    [EPublishStatushChanges.Closed]: EPublishStatus.Stopped,
}
