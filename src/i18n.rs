use actix_web::HttpRequest;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    Ru,
}

pub fn detect_locale(req: &HttpRequest) -> Locale {
    if let Some(lang) = req.query_string().split('&').find_map(|kv| {
        let mut it = kv.splitn(2, '=');
        let k = it.next()?;
        let v = it.next()?;
        if k == "lang" { Some(v) } else { None }
    }) {
        return match lang.to_ascii_lowercase().as_str() { "ru" | "ru-ru" => Locale::Ru, _ => Locale::En };
    }

    if let Some(h) = req.headers().get("Accept-Language").and_then(|v| v.to_str().ok()) {
        let hl = h.to_ascii_lowercase();
        if hl.starts_with("ru") { return Locale::Ru; }
    }

    Locale::En
}

pub fn direction_label(locale: Locale, dir: &str) -> Cow<'static, str> {
    match (locale, dir) {
        (Locale::Ru, "growing") => Cow::Borrowed("рост"),
        (Locale::Ru, "decreasing") => Cow::Borrowed("снижение"),
        (_, "growing") => Cow::Borrowed("growing"),
        (_, "decreasing") => Cow::Borrowed("decreasing"),
        _ => Cow::Owned(dir.to_string()),
    }
}
