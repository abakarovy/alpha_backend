use actix_web::{web, HttpResponse};
use serde_json::json;

pub async fn get_categories() -> HttpResponse {
    let categories = vec![
        json!({
            "id": "legal",
            "name": "Юридические вопросы",
            "description": "Регистрация, налоги, договоры, трудовое право",
            "icon": "⚖️"
        }),
        json!({
            "id": "marketing", 
            "name": "Маркетинг и продажи",
            "description": "Продвижение, SMM, таргетинг, аналитика",
            "icon": "📊"
        }),
        json!({
            "id": "finance",
            "name": "Финансы", 
            "description": "Учет, планирование, оптимизация расходов",
            "icon": "💰"
        }),
        json!({
            "id": "management",
            "name": "Управление",
            "description": "Персонал, процессы, масштабирование",
            "icon": "👥"
        }),
        json!({
            "id": "general",
            "name": "Общие вопросы",
            "description": "Разные бизнес-вопросы", 
            "icon": "💼"
        })
    ];
    
    HttpResponse::Ok().json(json!({
        "categories": categories
    }))
}

pub async fn get_resources(
    path: web::Path<String>,
) -> HttpResponse {
    let category = path.into_inner();
    
    let resources: serde_json::Value = match category.as_str() {
        "legal" => json!([
            {
                "title": "Регистрация бизнеса",
                "type": "guide",
                "description": "Пошаговое руководство по выбору формы собственности"
            },
            {
                "title": "Налоговые обязательства",
                "type": "checklist", 
                "description": "Список обязательных налогов и сроков уплаты"
            }
        ]),
        "marketing" => json!([
            {
                "title": "SMM стратегия",
                "type": "template",
                "description": "Готовый план продвижения в социальных сетях"
            },
            {
                "title": "Целевая аудитория",
                "type": "worksheet",
                "description": "Анкета для определения портрета клиента"
            }
        ]),
        "finance" => json!([
            {
                "title": "Финансовый план",
                "type": "template",
                "description": "Шаблон для финансового планирования"
            },
            {
                "title": "Отслеживание расходов",
                "type": "checklist",
                "description": "Чек-лист для контроля затрат"
            }
        ]),
        _ => json!([])
    };
    
    HttpResponse::Ok().json(json!({
        "category": category,
        "resources": resources
    }))
}