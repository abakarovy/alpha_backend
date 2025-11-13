use crate::state::AppState;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequestBody {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatResponseBody {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

pub async fn generate_response(
    message: &str,
    category: &str,
    business_type: &str,
    _state: &AppState,
    _user_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let model = std::env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "openrouter/auto".to_string());
    
    let system_prompt = get_system_prompt(category, business_type);

    let req_body = ChatRequestBody {
        model,
        messages: vec![
            ChatMessage { role: "system".to_string(), content: system_prompt },
            ChatMessage { role: "user".to_string(), content: message.to_string() },
        ],
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    let mut req = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&req_body);

    if let Ok(referer) = std::env::var("OPENROUTER_HTTP_REFERER") {
        req = req.header("HTTP-Referer", referer);
    }
    if let Ok(title) = std::env::var("OPENROUTER_APP_TITLE") {
        req = req.header("X-Title", title);
    }

    let res = match req.send().await {
        Ok(r) => r,
        Err(err) => {
            eprintln!("OpenRouter request failed to send: {}", err);
            return Err(err.into());
        }
    };

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        eprintln!("OpenRouter non-success status: {} body: {}", status, text);
        return Err(format!("OpenRouter request failed: {} - {}", status, text).into());
    }

    let body: ChatResponseBody = res.json().await?;
    let content = body
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_else(|| "".to_string());

    if content.is_empty() {
        return Err("Empty response from OpenRouter".into());
    }

    Ok(content)
}

fn get_system_prompt(category: &str, business_type: &str) -> String {
    let base_prompt = format!("Ты - опытный бизнес-консультант, помогающий владельцам малого бизнеса. Пользователь владеет бизнесом в сфере: {}. Отвечай профессионально, но доступно. Давай практические, реализуемые советы. Не пиши с новой строки (
    n), если пишешь какие-то конструкции (таблица, график, и т.д.), то окружай их в закругленных скобках, и перед открытием этих скобок пишешь большими буквами название конструкции", business_type);

    match category {
        "legal" => format!("{} Консультируй по юридическим вопросам: регистрация, налоги, договоры, трудовое право. Важно: уточняй, что это общие рекомендации и нужно консультироваться с юристом.", base_prompt),
        "marketing" => format!("{} Помогай с маркетингом: продвижение, SMM, таргетинг, брендинг, аналитика. Давай конкретные инструменты и стратегии.", base_prompt),
        "finance" => format!("{} Консультируй по финансам: учет, планирование, оптимизация расходов, налоговая оптимизация. Предлагай практические методы финансового управления.", base_prompt),
        _ => format!("{} Помогай с общими бизнес-вопросами: управление, найм, масштабирование, клиентский сервис.", base_prompt)
    }
}

fn generate_ai_advice(category: &str, business_type: &str) -> String {
    match category {
        "legal" => format!("Для бизнеса в сфере {} рекомендую:\n1. Проверить необходимые лицензии\n2. Составить типовые договоры\n3. Изучить налоговые обязательства\n\nПомните: это общие рекомендации, для точного ответа проконсультируйтесь с юристом.", business_type),
        "marketing" => format!("Для продвижения {}:\n1. Создайте аккаунты в релевантных соцсетях\n2. Настройте таргетированную рекламу\n3. Собирайте и анализируйте отзывы клиентов\n4. Разработайте программу лояльности", business_type),
        "finance" => format!("Финансовые советы для {}:\n1. Ведите ежедневный учет доходов/расходов\n2. Создайте финансовую подушку безопасности\n3. Автоматизируйте налоговую отчетность\n4. Планируйте бюджет на 3-6 месяцев вперед", business_type),
        _ => format!("Для развития бизнеса в сфере {}:\n1. Анализируйте конкурентов\n2. Собирайте обратную связь от клиентов\n3. Постоянно обучайтесь новым методикам\n4. Планируйте масштабирование бизнеса", business_type)
    }
}

pub async fn generate_quick_advice(category: &str, business_type: &str) -> Vec<String> {
    match category {
        "legal" => vec![
            format!("Проверьте лицензии для {}", business_type),
            "Составьте типовые договоры".to_string(),
            "Изучите налоговые обязательства".to_string(),
        ],
        "marketing" => vec![
            format!("Создайте аккаунты в соцсетях для {}", business_type),
            "Настройте таргетированную рекламу".to_string(),
            "Соберите базу клиентов".to_string(),
        ],
        "finance" => vec![
            "Ведите ежедневный учет доходов/расходов".to_string(),
            "Создайте финансовую подушку".to_string(),
            "Автоматизируйте отчетность".to_string(),
        ],
        _ => vec![
            "Анализируйте конкурентов".to_string(),
            "Собирайте отзывы клиентов".to_string(),
            "Планируйте развитие бизнеса".to_string(),
        ]
    }
}