use actix_web::{HttpResponse};

// MD into the binary at compile time
const PRIVACY_MD: &str = include_str!("../../assets/privacy_policy.md");

pub async fn privacy_policy() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/markdown; charset=utf-8")
        .body(PRIVACY_MD)
}
