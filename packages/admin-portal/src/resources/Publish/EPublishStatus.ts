// export enum EPublishStatus {
//     Void = 0,
//     Started = 1,
//     StartedLoading = 1.1,
//     Paused = 2,
//     PausedLoading = 2.1,
//     Stopped = 3,
//     StoppedLoading = 3.1,
//     Published = 4,
//     PublishedLoading = 4.1,
//     Generated = 5,
//     GeneratedLoading = 5.1,
// }

export enum EPublishStatus {
    Void = "VOID",
    Started = "STARTED",
    StartedLoading = "STARTEDLOADING",
    Paused = "PAUSED",
    PausedLoading = "PAUSEDLOADING",
    Stopped = "STOPPED",
    StoppedLoading = "STOPPEDLOADING",
    Published = "PUBLISHED",
    PublishedLoading = "PUBLISHEDLOADING",
    Generated = "GENERATED",
    GeneratedLoading = "GENERATEDLOADING",
}

export enum EPublishStatushChanges {
    Open = "OPEN",
    Paused = "PAUSED",
    Closed = "CLOSED",
    NotStarted = "NOT_STARTED",
}

export const PUBLISH_STATUS_CONVERT: {[key: string]: string} = {
    [EPublishStatushChanges.NotStarted]: EPublishStatus.Void,
    [EPublishStatushChanges.Open]: EPublishStatus.Started,
    [EPublishStatushChanges.Paused]: EPublishStatus.Paused,
    [EPublishStatushChanges.Closed]: EPublishStatus.Stopped,
}
