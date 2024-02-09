export interface ISchedule {
    id: string
    name: string
    date: Date
}

export const SCHEDULE_NAMES_LIST: Array<string> = [
    "System lockdown for finalization of Internet voting settings",
    "Start and end of pre-registration for OVCS",
    "Start and end of test voting period",
    "Start and end of Internet voting period",
    "Date and time covering the schedule for the following activities:",
    "Lab test",
    "Field test",
    "Mock elections",
    "FTS",
]
