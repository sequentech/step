export enum EPublishStatus {
    Void = 0,
    Started = 1,
    StartedLoading = 1.1,
    Paused = 2,
    PausedLoading = 2.1,
    Stopped = 3,
    StoppedLoading = 3.1,
    Published = 4,
    PublishedLoading = 4.1,
    Generated = 5,
    GeneratedLoading = 5.1,
}

export enum EPublishStatushChanges {
    Open = 'OPEN',
    Paused = 'PAUSED',
    Closed = 'CLOSED'
}

export const PUBLICH_STATUS_CONVERT: {[key: string]: number} = {
    [EPublishStatushChanges.Open]: EPublishStatus.Started,
    [EPublishStatushChanges.Paused]: EPublishStatus.Paused,
    [EPublishStatushChanges.Closed]: EPublishStatus.Stopped
}