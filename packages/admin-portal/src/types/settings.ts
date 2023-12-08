export type TTenant = {
    setting: {
        spanish: boolean;
        english: boolean;
        sms: boolean;
        mail: boolean;
    };
    voting_channels: {
        online: boolean;
        kiosk: boolean;
    };
}