// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Аутентификация для подключения к официальным серверам Minecraft (Java Edition).
//! Цепочка:
//!   1. Microsoft OAuth2 — получение кода через браузер
//!   2. Обмен кода на Microsoft access token
//!   3. Xbox Live authenticate
//!   4. XSTS authorize
//!   5. Minecraft access token + профиль (UUID, имя)

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Результат полной цепочки аутентификации.
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub access_token: String,   // Minecraft access token (Bearer)
    pub player_uuid: String,    // UUID игрока без тире
    pub player_name: String,    // Имя игрока
    pub xuid: String,           // Xbox Live XUID
    pub expires_in: i64,        // Секунды до истечения токена
}

/// Ошибки аутентификации.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Auth failed: {0}")]
    Failed(String),
    #[error("Missing field: {0}")]
    MissingField(String),
}

/// Клиент аутентификации Microsoft / Minecraft.
pub struct MinecraftAuthClient {
    http: Client,
    client_id: String,
    redirect_uri: String,
}

impl MinecraftAuthClient {
    /// Создать клиент с client_id приложения Azure AD и redirect_uri.
    pub fn new(client_id: &str, redirect_uri: &str) -> Self {
        Self {
            http: Client::new(),
            client_id: client_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
        }
    }

    /// URL для открытия в браузере. Пользователь логинится и возвращается на redirect_uri с параметром `?code=...`.
    pub fn login_url(&self, state: &str) -> String {
        format!(
            "https://login.live.com/oauth20_authorize.srf?client_id={}&redirect_uri={}&response_type=code&scope=service::user.auth.xboxlive.user::read&state={}",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            state,
        )
    }

    /// Полный пайплайн: обмен кода авторизации на Minecraft access token и профиль.
    pub fn authenticate(&self, auth_code: &str) -> Result<AuthResult, AuthError> {
        let microsoft_token = self.exchange_microsoft_code(auth_code)?;
        let (xbox_token, uhs) = self.xbox_authenticate(&microsoft_token.access_token)?;
        let (xsts_token, uhs2) = self.xsts_authorize(&xbox_token)?;
        let _ = uhs;
        let minecraft = self.minecraft_login(&xsts_token, &uhs2)?;
        let profile = self.minecraft_profile(&minecraft.access_token)?;

        Ok(AuthResult {
            access_token: minecraft.access_token,
            player_uuid: profile.id.replace('-', ""),
            player_name: profile.name,
            xuid: uhs2,
            expires_in: minecraft.expires_in,
        })
    }

    // ---- Шаг 2: обмен кода на Microsoft токен ----
    fn exchange_microsoft_code(&self, auth_code: &str) -> Result<MicrosoftTokenResponse, AuthError> {
        let mut params = HashMap::new();
        params.insert("client_id", self.client_id.as_str());
        params.insert("redirect_uri", self.redirect_uri.as_str());
        params.insert("client_secret", "");
        params.insert("code", auth_code);
        params.insert("grant_type", "authorization_code");

        let resp: MicrosoftTokenResponse = self
            .http
            .post("https://login.live.com/oauth20_token.srf")
            .form(&params)
            .send()?
            .error_for_status()?
            .json()?;

        Ok(resp)
    }

    // ---- Шаг 3: Xbox Live authenticate ----
    fn xbox_authenticate(&self, access_token: &str) -> Result<(String, String), AuthError> {
        let body = XboxAuthenticateRequest {
            properties: XboxAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: format!("d={}", access_token),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        };

        let resp: XboxAuthenticateResponse = self
            .http
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&body)
            .send()?
            .error_for_status()?
            .json()?;

        let uhs = resp
            .display_claims
            .xui
            .first()
            .ok_or_else(|| AuthError::MissingField("xui".to_string()))?
            .uhs
            .clone();

        Ok((resp.token, uhs))
    }

    // ---- Шаг 4: XSTS authorize ----
    fn xsts_authorize(&self, xbox_token: &str) -> Result<(String, String), AuthError> {
        let body = XstsAuthorizeRequest {
            properties: XstsAuthProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbox_token.to_string()],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        };

        let resp: XstsAuthorizeResponse = self
            .http
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&body)
            .send()?
            .error_for_status()?
            .json()?;

        let uhs = resp
            .display_claims
            .xui
            .first()
            .ok_or_else(|| AuthError::MissingField("xui".to_string()))?
            .uhs
            .clone();

        Ok((resp.token, uhs))
    }

    // ---- Шаг 5: Minecraft login with Xbox ----
    fn minecraft_login(&self, xsts_token: &str, uhs: &str) -> Result<MinecraftTokenResponse, AuthError> {
        let body = MinecraftLoginRequest {
            identity_token: format!("XBL3.0 x={};{}", uhs, xsts_token),
        };

        let resp: MinecraftTokenResponse = self
            .http
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .json(&body)
            .send()?
            .error_for_status()?
            .json()?;

        Ok(resp)
    }

    // ---- Шаг 6: Minecraft profile ----
    fn minecraft_profile(&self, access_token: &str) -> Result<MinecraftProfile, AuthError> {
        let resp: MinecraftProfile = self
            .http
            .get("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(access_token)
            .send()?
            .error_for_status()?
            .json()?;

        Ok(resp)
    }
}

// ---- Структуры запросов/ответов ----

#[derive(Deserialize)]
struct MicrosoftTokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    #[allow(dead_code)]
    pub token_type: String,
}

#[derive(Serialize)]
struct XboxAuthenticateRequest {
    properties: XboxAuthProperties,
    relying_party: String,
    token_type: String,
}

#[derive(Serialize)]
struct XboxAuthProperties {
    auth_method: String,
    site_name: String,
    rps_ticket: String,
}

#[derive(Deserialize)]
struct XboxAuthenticateResponse {
    pub token: String,
    #[allow(dead_code)]
    pub issue_instant: String,
    #[allow(dead_code)]
    pub not_after: String,
    pub display_claims: XboxDisplayClaims,
}

#[derive(Deserialize)]
struct XboxDisplayClaims {
    pub xui: Vec<XuiClaim>,
}

#[derive(Deserialize)]
struct XuiClaim {
    pub uhs: String,
}

#[derive(Serialize)]
struct XstsAuthorizeRequest {
    properties: XstsAuthProperties,
    relying_party: String,
    token_type: String,
}

#[derive(Serialize)]
struct XstsAuthProperties {
    sandbox_id: String,
    user_tokens: Vec<String>,
}

#[derive(Deserialize)]
struct XstsAuthorizeResponse {
    pub token: String,
    #[allow(dead_code)]
    pub issue_instant: String,
    #[allow(dead_code)]
    pub not_after: String,
    pub display_claims: XstsDisplayClaims,
}

#[derive(Deserialize)]
struct XstsDisplayClaims {
    pub xui: Vec<XstsXuiClaim>,
}

#[derive(Deserialize)]
struct XstsXuiClaim {
    pub uhs: String,
}

#[derive(Serialize)]
struct MinecraftLoginRequest {
    identity_token: String,
}

#[derive(Deserialize)]
struct MinecraftTokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    #[allow(dead_code)]
    pub token_type: String,
}

#[derive(Deserialize)]
struct MinecraftProfile {
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub skins: Vec<MinecraftSkin>,
    #[allow(dead_code)]
    pub capes: Vec<MinecraftCape>,
}

#[derive(Deserialize)]
struct MinecraftSkin {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub state: String,
    #[allow(dead_code)]
    pub url: String,
    #[allow(dead_code)]
    pub variant: String,
}

#[derive(Deserialize)]
struct MinecraftCape {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub state: String,
    #[allow(dead_code)]
    pub url: String,
}
