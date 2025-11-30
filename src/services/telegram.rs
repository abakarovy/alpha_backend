use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize)]
struct SendMessageRequest {
    chat_id: i64,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<String>,
}

#[derive(Serialize)]
struct SendPhotoRequest {
    chat_id: i64,
    photo: String, // file_id or URL
    #[serde(skip_serializing_if = "Option::is_none")]
    caption: Option<String>,
}

#[derive(Deserialize)]
struct TelegramResponse {
    ok: bool,
    #[serde(default)]
    result: Option<TelegramMessageResult>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct TelegramMessageResult {
    message_id: i64,
}

pub struct TelegramBot {
    client: Client,
    bot_token: String,
    group_chat_id: i64,
    api_url: String,
}

impl TelegramBot {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let bot_token = env::var("TELEGRAM_BOT_TOKEN")?;
        let group_chat_id: i64 = env::var("TELEGRAM_GROUP_CHAT_ID")?
            .parse()
            .map_err(|_| "Invalid TELEGRAM_GROUP_CHAT_ID")?;
        
        let api_url = format!("https://api.telegram.org/bot{}", bot_token);
        
        Ok(TelegramBot {
            client: Client::new(),
            bot_token,
            group_chat_id,
            api_url,
        })
    }

    pub async fn send_message(
        &self,
        text: &str,
        user_name: Option<&str>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let message = if let Some(name) = user_name {
            format!("👤 <b>{}</b>\n\n{}", name, text)
        } else {
            format!("👤 <b>Пользователь</b>\n\n{}", text)
        };

        let request = SendMessageRequest {
            chat_id: self.group_chat_id,
            text: message,
            parse_mode: Some("HTML".to_string()),
        };

        let url = format!("{}/sendMessage", self.api_url);
        let response_text = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .text()
            .await?;
        
        let response: TelegramResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse Telegram response: {}", e))?;

        if response.ok {
            if let Some(msg) = response.result {
                Ok(msg.message_id)
            } else {
                Err("No message ID in response".into())
            }
        } else {
            Err(format!("Telegram API error: {:?}", response.description).into())
        }
    }

    pub async fn send_photo(
        &self,
        photo_url: &str,
        caption: Option<&str>,
        user_name: Option<&str>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let caption_text = if let Some(name) = user_name {
            if let Some(cap) = caption {
                format!("👤 <b>{}</b>\n\n{}", name, cap)
            } else {
                format!("👤 <b>{}</b>", name)
            }
        } else {
            caption.unwrap_or("👤 <b>Пользователь</b>").to_string()
        };

        let request = SendPhotoRequest {
            chat_id: self.group_chat_id,
            photo: photo_url.to_string(),
            caption: Some(caption_text),
        };

        let url = format!("{}/sendPhoto", self.api_url);
        let response_text = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .text()
            .await?;
        
        let response: TelegramResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse Telegram response: {}", e))?;

        if response.ok {
            if let Some(msg) = response.result {
                Ok(msg.message_id)
            } else {
                Err("No message ID in response".into())
            }
        } else {
            Err(format!("Telegram API error: {:?}", response.description).into())
        }
    }

    pub async fn send_photo_multipart(
        &self,
        photo_data: Vec<u8>,
        filename: &str,
        caption: Option<&str>,
        user_name: Option<&str>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let caption_text = if let Some(name) = user_name {
            if let Some(cap) = caption {
                format!("👤 {}\n\n{}", name, cap)
            } else {
                format!("👤 {}", name)
            }
        } else {
            caption.unwrap_or("👤 Пользователь").to_string()
        };

        let url = format!("{}/sendPhoto", self.api_url);
        
        let form = reqwest::multipart::Form::new()
            .text("chat_id", self.group_chat_id.to_string())
            .text("caption", caption_text)
            .part(
                "photo",
                reqwest::multipart::Part::bytes(photo_data)
                    .file_name(filename.to_string())
                    .mime_str("image/jpeg")?,
            );

        let response_text = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await?
            .text()
            .await?;
        
        let response: TelegramResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse Telegram response: {}", e))?;

        if response.ok {
            if let Some(msg) = response.result {
                Ok(msg.message_id)
            } else {
                Err("No message ID in response".into())
            }
        } else {
            Err(format!("Telegram API error: {:?}", response.description).into())
        }
    }
}

