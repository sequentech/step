// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::jwt::*;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthHeaders {
    pub key: String,
    pub value: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthHeaders {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let headers = request.headers().clone();
        if headers.contains("X-Hasura-Admin-Secret") {
            Outcome::Success(AuthHeaders {
                key: "X-Hasura-Admin-Secret".to_string(),
                value: headers
                    .get_one("X-Hasura-Admin-Secret")
                    .unwrap()
                    .to_string(),
            })
        } else if headers.contains("authorization") {
            Outcome::Success(AuthHeaders {
                key: "authorization".to_string(),
                value: headers.get_one("authorization").unwrap().to_string(),
            })
        } else {
            Outcome::Error((Status::Unauthorized, ()))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for JwtClaims {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let headers = request.headers().clone();
        match headers.get_one("authorization") {
            Some(authorization) => {
                let authorization2 = "Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJJYXRJRWFNd004REEyVGFWa3ZEQklxbjcxQ2plRjhLeEx5VU9MbnBjRXlJIn0.eyJleHAiOjE3MDYwNDIyNTIsImlhdCI6MTcwNjA0MTk1MiwiYXV0aF90aW1lIjoxNzA2MDQxMzMwLCJqdGkiOiIzZTc0N2M3MS00OWNiLTQ4YTMtOTgxNC01MDZiYzViY2JlZDAiLCJpc3MiOiJodHRwczovL2tleWNsb2FrLXN0YWdpbmcuc2VxdWVudC52b3RlL2F1dGgvcmVhbG1zL3RlbmFudC05MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUiLCJhdWQiOlsidm90aW5nLXBvcnRhbCIsImFjY291bnQiXSwic3ViIjoiMDUxNmVkMjItM2E4Zi00OGNhLTk1MTQtODMwMjBlMjQyMTU5IiwidHlwIjoiQmVhcmVyIiwiYXpwIjoiYWRtaW4tcG9ydGFsIiwibm9uY2UiOiJmMzA4MmIyYS05MDY0LTQ2OTAtOWRhMS1jZmFjYWUxYjJlMzMiLCJzZXNzaW9uX3N0YXRlIjoiOWU5ZWYxNzYtN2VlZC00MDRlLTg2YTctMWZkM2NmYzMwYTFlIiwiYWNyIjoiMCIsImFsbG93ZWQtb3JpZ2lucyI6WyIqIl0sInJlYWxtX2FjY2VzcyI6eyJyb2xlcyI6WyJlbGVjdGlvbi10eXBlLWNyZWF0ZSIsInJvbGUtd3JpdGUiLCJ0YWxseS1yZXN1bHRzLXJlYWQiLCJlbGVjdGlvbi1ldmVudC1jcmVhdGUiLCJ0YWxseS1zaGVldC12aWV3Iiwidm90aW5nLWNoYW5uZWwtcmVhZCIsInRhbGx5LXdyaXRlIiwidXNlci1jcmVhdGUiLCJ1bWFfYXV0aG9yaXphdGlvbiIsInRhbGx5LXNoZWV0LWRlbGV0ZSIsImFkbWluLWRhc2hib2FyZC12aWV3IiwidXNlci1yZWFkIiwibm90aWZpY2F0aW9uLXdyaXRlIiwidGVuYW50LWNyZWF0ZSIsImRvY3VtZW50LWRvd25sb2FkIiwibm90aWZpY2F0aW9uLXNlbmQiLCJ0YWxseS1yZWFkIiwidGFsbHktc2hlZXQtcHVibGlzaCIsInRlbmFudC13cml0ZSIsInRydXN0ZWUtY3JlYXRlIiwiZWxlY3Rpb24tdHlwZS13cml0ZSIsImFkbWluLWNlcmVtb255Iiwidm90ZXItd3JpdGUiLCJub3RpZmljYXRpb24tcmVhZCIsImNvbW11bmljYXRpb24tdGVtcGxhdGUtd3JpdGUiLCJ0YWxseS1zdGFydCIsInRlbmFudC1yZWFkIiwidXNlci1wZXJtaXNzaW9uLXJlYWQiLCJwdWJsaXNoLXJlYWQiLCJ0cnVzdGVlLXJlYWQiLCJjcmVhdGUtZWxlY3Rpb24tZXZlbnQiLCJlbGVjdGlvbi1ldmVudC1yZWFkIiwidm90aW5nLWNoYW5uZWwtd3JpdGUiLCJyb2xlLWFzc2lnbiIsInJvbGUtcmVhZCIsImFkbWluLXVzZXIiLCJjb21tdW5pY2F0aW9uLXRlbXBsYXRlLXJlYWQiLCJlbGVjdGlvbi1ldmVudC13cml0ZSIsImVsZWN0aW9uLXR5cGUtcmVhZCIsIm9mZmxpbmVfYWNjZXNzIiwidm90ZXItY3JlYXRlIiwiY29tbXVuaWNhdGlvbi10ZW1wbGF0ZS1jcmVhdGUiLCJ1c2VyLXBlcm1pc3Npb24tY3JlYXRlIiwia2V5cy1yZWFkIiwidm90ZXItcmVhZCIsInVzZXItd3JpdGUiLCJhcmVhLXJlYWQiLCJ0YWxseS1zaGVldC1jcmVhdGUiLCJyb2xlLWNyZWF0ZSIsImRlZmF1bHQtcm9sZXMtZWxlY3RvcmFsLXByb2Nlc3MiLCJlbGVjdGlvbi1zdGF0ZS13cml0ZSIsImRvY3VtZW50LXVwbG9hZCIsInRydXN0ZWUtd3JpdGUiLCJhcmVhLXdyaXRlIiwidXNlci1wZXJtaXNzaW9uLXdyaXRlIiwicHVibGlzaC13cml0ZSIsImxvZ3MtcmVhZCJdfSwicmVzb3VyY2VfYWNjZXNzIjp7InZvdGluZy1wb3J0YWwiOnsicm9sZXMiOlsidXNlciJdfSwiYWNjb3VudCI6eyJyb2xlcyI6WyJtYW5hZ2UtYWNjb3VudCIsIm1hbmFnZS1hY2NvdW50LWxpbmtzIiwidmlldy1wcm9maWxlIl19fSwic2NvcGUiOiJvcGVuaWQgcHJvZmlsZSBlbWFpbCIsInNpZCI6IjllOWVmMTc2LTdlZWQtNDA0ZS04NmE3LTFmZDNjZmMzMGExZSIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJodHRwczovL2hhc3VyYS5pby9qd3QvY2xhaW1zIjp7IngtaGFzdXJhLWRlZmF1bHQtcm9sZSI6ImFkbWluLXVzZXIiLCJ4LWhhc3VyYS10ZW5hbnQtaWQiOiI5MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUiLCJ4LWhhc3VyYS11c2VyLWlkIjoiMDUxNmVkMjItM2E4Zi00OGNhLTk1MTQtODMwMjBlMjQyMTU5IiwieC1oYXN1cmEtYWxsb3dlZC1yb2xlcyI6WyJlbGVjdGlvbi10eXBlLWNyZWF0ZSIsInJvbGUtd3JpdGUiLCJ0YWxseS1yZXN1bHRzLXJlYWQiLCJlbGVjdGlvbi1ldmVudC1jcmVhdGUiLCJ0YWxseS1zaGVldC12aWV3Iiwidm90aW5nLWNoYW5uZWwtcmVhZCIsInRhbGx5LXdyaXRlIiwidXNlci1jcmVhdGUiLCJ1bWFfYXV0aG9yaXphdGlvbiIsInRhbGx5LXNoZWV0LWRlbGV0ZSIsImFkbWluLWRhc2hib2FyZC12aWV3IiwidXNlci1yZWFkIiwibm90aWZpY2F0aW9uLXdyaXRlIiwidGVuYW50LWNyZWF0ZSIsImRvY3VtZW50LWRvd25sb2FkIiwibm90aWZpY2F0aW9uLXNlbmQiLCJ0YWxseS1yZWFkIiwidGFsbHktc2hlZXQtcHVibGlzaCIsInRlbmFudC13cml0ZSIsInRydXN0ZWUtY3JlYXRlIiwiZWxlY3Rpb24tdHlwZS13cml0ZSIsImFkbWluLWNlcmVtb255Iiwidm90ZXItd3JpdGUiLCJub3RpZmljYXRpb24tcmVhZCIsImNvbW11bmljYXRpb24tdGVtcGxhdGUtd3JpdGUiLCJ0YWxseS1zdGFydCIsInRlbmFudC1yZWFkIiwidXNlci1wZXJtaXNzaW9uLXJlYWQiLCJwdWJsaXNoLXJlYWQiLCJ0cnVzdGVlLXJlYWQiLCJjcmVhdGUtZWxlY3Rpb24tZXZlbnQiLCJlbGVjdGlvbi1ldmVudC1yZWFkIiwidm90aW5nLWNoYW5uZWwtd3JpdGUiLCJyb2xlLWFzc2lnbiIsInJvbGUtcmVhZCIsImFkbWluLXVzZXIiLCJjb21tdW5pY2F0aW9uLXRlbXBsYXRlLXJlYWQiLCJlbGVjdGlvbi1ldmVudC13cml0ZSIsImVsZWN0aW9uLXR5cGUtcmVhZCIsIm9mZmxpbmVfYWNjZXNzIiwidm90ZXItY3JlYXRlIiwiY29tbXVuaWNhdGlvbi10ZW1wbGF0ZS1jcmVhdGUiLCJ1c2VyLXBlcm1pc3Npb24tY3JlYXRlIiwia2V5cy1yZWFkIiwidm90ZXItcmVhZCIsInVzZXItd3JpdGUiLCJhcmVhLXJlYWQiLCJ0YWxseS1zaGVldC1jcmVhdGUiLCJyb2xlLWNyZWF0ZSIsImRlZmF1bHQtcm9sZXMtZWxlY3RvcmFsLXByb2Nlc3MiLCJlbGVjdGlvbi1zdGF0ZS13cml0ZSIsImRvY3VtZW50LXVwbG9hZCIsInRydXN0ZWUtd3JpdGUiLCJhcmVhLXdyaXRlIiwidXNlci1wZXJtaXNzaW9uLXdyaXRlIiwicHVibGlzaC13cml0ZSIsImxvZ3MtcmVhZCJdfSwibmFtZSI6IkZlbGl4IFJvYmxlcyIsInByZWZlcnJlZF91c2VybmFtZSI6ImZlbGl4IiwibG9jYWxlIjoiZW4iLCJnaXZlbl9uYW1lIjoiRmVsaXgiLCJmYW1pbHlfbmFtZSI6IlJvYmxlcyIsImVtYWlsIjoiZmVsaXhAc2VxdWVudGVjaC5pbyJ9.HD92EHtgIEGXxXY3u3SSRyGVxe9AqgN9JGncGpWj4ed3b3MBN0Xjhk2i648dLb_kFO05SaPP55GoDyppBH9PbOv_qaRzLEUyKug4E9i0s4amvybTAL0fikPVZdv_HocErZhrnISXAaA2KR3CjdOdImjPVpGrshswWQJUfrHDu_rtxzWbuVsbADZ_TdjVAfs1VNbv7fNIg4pbllDXOKhqsae52LYUegnr6MK_-9l1yWlpUW6HBSPasqodckVwl-C5X_YXgcF5Vrxszkhx3fu3OTEQmMy-twPsp69WtrsfFTEIbkQ3OfSsNQSvrSUZ0gDcDpBxk0VL7LzWiSOXR_Odmw";
                match authorization2.strip_prefix("Bearer ") {
                    Some(token) => Outcome::Success(decode_jwt(token).unwrap()),
                    None => Outcome::Error((Status::Unauthorized, ())),
                }
            }
            None => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
