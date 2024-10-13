use crate::services::authorization::authorize;
use sequent_core::types::permissions::Permissions;

pub struct GenerateReportBody {
    pub report_id: String
}

pub struct GenerateReportResponse {
    pub document_id: String
}


#[instrument(skip(claims))]
#[post("/generate-report", format = "json", data = "<body>")]
pub async fn generate_report(
    claims: jwt::JwtClaims,
    body: Json<GenerateReportBody>,
) -> Result<Json<GenerateReportResponse>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )?; 
    
    Ok(Json(report))
}
