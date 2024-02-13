export interface ISchedule {
    id: string
    name: string
    date: Date
}

export enum ScheduledEvents {
    SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS = "SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS",
    START_PRE_REGISTRATION_OVCS = "START_PRE_REGISTRATION_OVCS",
    END_PRE_REGISTRATION_OVCS = "END_PRE_REGISTRATION_OVCS",
    START_TEST_VOTING_PERIOD = "START_TEST_VOTING_PERIOD",
    END_TEST_VOTING_PERIOD = "END_TEST_VOTING_PERIOD",
    START_INTERNET_VOTING_PERIOD = "START_INTERNET_VOTING_PERIOD",
    END_INTERNET_VOTING_PERIOD = "END_INTERNET_VOTING_PERIOD",
    LAB_TEST = "LAB_TEST",
    FIELD_TEST = "FIELD_TEST",
    MOCK_ELECTIONS = "MOCK_ELECTIONS",
    FTS = "FTS",
}

export const SCHEDULE_NAMES_LIST: Array<string> = [
    "System lockdown for finalization of Internet voting settings",
    "Start and end of pre-registration for OVCS",
    "Start and end of test voting period",
    "Start and end of Internet voting period",
    "Lab test",
    "Field test",
    "Mock elections",
    "FTS",
]
