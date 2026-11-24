//! OAuth 2.0 によるアクセストークン取得のモジュール境界。
//!
//! 本実装（リフレッシュトークンでのトークン更新、`invalid_grant` の検出など）は次 PR で行う。
//! リフレッシュトークンの失効は `AuthError::RefreshTokenExpired` として返し、終了コード 2 になる。

use std::fmt;

use crate::config::GoogleCredentials;
use crate::error::AuthError;
use crate::Result;

/// Google OAuth 2.0 のトークンエンドポイント。
pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

/// Health API 呼び出しに使う Bearer アクセストークン。
#[derive(Clone)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken(***)")
    }
}

/// リフレッシュトークンからアクセストークンを取得する（スタブ）。
pub async fn obtain_access_token(_credentials: GoogleCredentials) -> Result<AccessToken> {
    Err(AuthError::NotImplemented.into())
}
