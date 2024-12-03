use lazy_static::lazy_static;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMessage {
    pub subject: String,
    pub plaintext_body: String,
    pub html_body: String,
    pub sms_body: String,
}

lazy_static! {
    pub static ref APPLICATION_MESSAGES: HashMap<String, ApplicationMessage> = {
        let mut m = HashMap::new();
        
        // Approval message
        m.insert(
            "approval".to_string(),
            ApplicationMessage {
                subject: "Application accepted".to_string(),
                plaintext_body: "Hello!\n\nYour application has been accepted successfully.\n\nYou can now use {email} as username to login and the provided password during registration.\n\nRegards,".to_string(),
                html_body: "Hello!<br><br>Your application has been accepted successfully.<br><br>You can now use {email} as username to login and the provided password during registration.<br><br>Regards,".to_string(),
                sms_body: "Your application has been accepted successfully. You can now use {phone_number} as username to login and the provided password during registration.".to_string(),
            }
        );

        // Rejection messages for different reasons
        m.insert(
            "insufficient-information".to_string(),
            ApplicationMessage {
                subject: "Application rejected - Insufficient Information".to_string(),
                plaintext_body: "Hello!\n\nYour application has been rejected due to insufficient information.\n\n{rejection_message}\n\nPlease submit a new application with complete information.\n\nRegards,".to_string(),
                html_body: "Hello!<br><br>Your application has been rejected due to insufficient information.<br><br>{rejection_message}<br><br>Please submit a new application with complete information.<br><br>Regards,".to_string(),
                sms_body: "Your application has been rejected due to insufficient information. {rejection_message} Please submit a new application with complete information.".to_string(),
            }
        );

        m.insert(
            "no-matching-voter".to_string(),
            ApplicationMessage {
                subject: "Application rejected - No Matching Voter Found".to_string(),
                plaintext_body: "Hello!\n\nYour application has been rejected as we could not find a matching voter record.\n\n{rejection_message}\n\nPlease contact support if you believe this is an error.\n\nRegards,".to_string(),
                html_body: "Hello!<br><br>Your application has been rejected as we could not find a matching voter record.<br><br>{rejection_message}<br><br>Please contact support if you believe this is an error.<br><br>Regards,".to_string(),
                sms_body: "Your application has been rejected as we could not find a matching voter record. {rejection_message} Please contact support if you believe this is an error.".to_string(),
            }
        );

        m.insert(
            "voter-already-approved".to_string(),
            ApplicationMessage {
                subject: "Application rejected - Voter Already Approved".to_string(),
                plaintext_body: "Hello!\n\nYour application has been rejected as this voter has already been approved.\n\n{rejection_message}\n\nIf you did not submit the previous application, please contact support immediately.\n\nRegards,".to_string(),
                html_body: "Hello!<br><br>Your application has been rejected as this voter has already been approved.<br><br>{rejection_message}<br><br>If you did not submit the previous application, please contact support immediately.<br><br>Regards,".to_string(),
                sms_body: "Your application has been rejected as this voter has already been approved. {rejection_message} If you did not submit the previous application, please contact support immediately.".to_string(),
            }
        );

        m.insert(
            "other".to_string(),
            ApplicationMessage {
                subject: "Application rejected".to_string(),
                plaintext_body: "Hello!\n\nYour application has been rejected.\n\n{rejection_message}\n\nRegards,".to_string(),
                html_body: "Hello!<br><br>Your application has been rejected.<br><br>{rejection_message}<br><br>Regards,".to_string(),
                sms_body: "Your application has been rejected. {rejection_message}".to_string(),
            }
        );

        m
    };
}

pub fn get_application_message(message_type: &str) -> Option<&'static ApplicationMessage> {
    APPLICATION_MESSAGES.get(message_type)
} 