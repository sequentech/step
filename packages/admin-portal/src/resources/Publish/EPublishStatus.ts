export enum PublishStatus {
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

export const nextStatus = (statusValue: PublishStatus): PublishStatus => {
    let statusIndex = Object.values(PublishStatus).indexOf(statusValue)
    return Object.values(PublishStatus)[statusIndex + 1]
}

export enum ElectionEventStatus {
    Open = "OPEN",
    Paused = "PAUSED",
    Closed = "CLOSED",
    NotStarted = "NOT_STARTED",
}

export const MAP_ELECTION_EVENT_STATUS_PUBLISH = {
    [ElectionEventStatus.NotStarted]: PublishStatus.Void,
    [ElectionEventStatus.Open]: PublishStatus.Started,
    [ElectionEventStatus.Paused]: PublishStatus.Paused,
    [ElectionEventStatus.Closed]: PublishStatus.Stopped,
}
