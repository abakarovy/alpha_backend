use std::env;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use base64::{Engine as _, engine::general_purpose};

#[derive(Deserialize, Debug)]
struct ServiceAccount {
    #[serde(rename = "type")]
    account_type: String,
    project_id: String,
    private_key_id: Option<String>,
    private_key: String,
    client_email: String,
    client_id: Option<String>,
    auth_uri: Option<String>,
    token_uri: String,
    auth_provider_x509_cert_url: Option<String>,
    client_x509_cert_url: Option<String>,
}

#[derive(Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

pub struct FcmService {
    client: Client,
    service_account: Option<ServiceAccount>,
    access_token: Arc<Mutex<Option<(String, u64)>>>, // (token, expiry_timestamp)
    project_id: Option<String>,
}

impl FcmService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::new();
        
        let service_account = if let Ok(json_str) = env::var("FCM_SERVICE_ACCOUNT_JSON") {
            // Service account JSON as environment variable (base64 encoded or plain JSON)
            let json_content = if json_str.starts_with('{') {
                json_str
            } else {
                String::from_utf8(general_purpose::STANDARD
                    .decode(&json_str)?)?
            };
            Some(serde_json::from_str::<ServiceAccount>(&json_content)?)
        } else if let Ok(file_path) = env::var("FCM_SERVICE_ACCOUNT_PATH") {
            let json_content = std::fs::read_to_string(&file_path)?;
            Some(serde_json::from_str::<ServiceAccount>(&json_content)?)
        } else if let Ok(google_creds) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            let json_content = std::fs::read_to_string(&google_creds)?;
            Some(serde_json::from_str::<ServiceAccount>(&json_content)?)
        } else {
            None
        };

        let project_id = service_account.as_ref().map(|sa| sa.project_id.clone());

        Ok(FcmService {
            client,
            service_account,
            access_token: Arc::new(Mutex::new(None)),
            project_id,
        })
    }

    async fn get_access_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        let service_account = match &self.service_account {
            Some(sa) => sa,
            None => {
                return Err("FCM service account not configured".into());
            }
        };

        {
            let token_lock = self.access_token.lock().unwrap();
            if let Some((token, expiry)) = &*token_lock {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)?
                    .as_secs();
                if *expiry > now + 300 {
                    return Ok(token.clone());
                }
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let claims = JwtClaims {
            iss: service_account.client_email.clone(),
            scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
            aud: service_account.token_uri.clone(),
            exp: now + 3600, // 1 hour
            iat: now,
        };

        let encoding_key = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())?;

        let jwt = encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)?;

        let token_response: TokenResponse = self.client
            .post(&service_account.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await?
            .json()
            .await?;

        let expiry = now + token_response.expires_in;
        {
            let mut token_lock = self.access_token.lock().unwrap();
            *token_lock = Some((token_response.access_token.clone(), expiry));
        }

        Ok(token_response.access_token)
    }

    pub async fn send_notification(
        &self,
        tokens: Vec<String>,
        title: &str,
        body: &str,
        data: Option<HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project_id = match &self.project_id {
            Some(id) => id,
            None => {
                eprintln!("FCM not configured - skipping push notifications");
                return Ok(());
            }
        };

        let access_token = self.get_access_token().await?;

        let url = format!("https://fcm.googleapis.com/v1/projects/{}/messages:send", project_id);

        for token in tokens {
            let mut message = json!({
                "message": {
                    "token": token,
                    "notification": {
                        "title": title,
                        "body": body
                    }
                }
            });

            if let Some(data_map) = &data {
                let mut data_obj = json!({});
                for (k, v) in data_map {
                    data_obj[k] = json!(v);
                }
                message["message"]["data"] = data_obj;
            }

            let response = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&message)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let error_text = resp.text().await.unwrap_or_default();
                        eprintln!("FCM v1 API error: {} - {}", status, error_text);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to send FCM notification: {}", e);
                }
            }
        }

        Ok(())
    }
}
