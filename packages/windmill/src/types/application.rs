use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames)]
pub enum ApplicationStatus {
    PENDING,
    ACCEPTED,
    REJECTED,
}

#[derive(Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames)]
pub enum ApplicationType {
    AUTOMATIC,
    MANUAL,
}
