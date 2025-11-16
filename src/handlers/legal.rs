use actix_web::{HttpRequest, HttpResponse};
use crate::i18n::{self, Locale};

// Embed EN and RU Markdown files
const PRIVACY_MD_EN: &str = include_str!("../../assets/privacy_policy.md");
const PRIVACY_MD_RU: &str = include_str!("../../assets/privacy_policy.ru.md");

pub async fn privacy_policy(req: HttpRequest) -> HttpResponse {
    let loc = i18n::detect_locale(&req);
    let body = match loc {
        Locale::Ru => PRIVACY_MD_RU,
        _ => PRIVACY_MD_EN,
    };
    HttpResponse::Ok()
        .content_type("text/markdown; charset=utf-8")
        .body(body)
}
