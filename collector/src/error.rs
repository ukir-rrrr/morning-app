use thiserror::Error;

pub const EXIT_OK: u8 = 0;
pub const EXIT_FAILURE: u8 = 1;
pub const EXIT_AUTH_EXPIRED: u8 = 2;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error(transparent)]
    Auth(#[from] AuthError),

    #[error("Health API returned HTTP {status}: {body}")]
    Api { status: u16, body: String },

    #[error("pagination error: {0}")]
    Pagination(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Db(#[from] libsql::Error),
}

/// 認証関連のエラー。`requires_reauth()` が真のものは終了コード 2 になる。
#[derive(Debug, Error)]
pub enum AuthError {
    /// リフレッシュトークンが失効・取り消し済み（`invalid_grant` 等）。再認証が必要。
    #[error("refresh token expired or revoked; re-authorization required")]
    RefreshTokenExpired,

    /// Health API が 401 を返した。
    #[error("access token rejected by Health API (HTTP 401)")]
    Unauthorized,

    /// OAuth の本実装は次 PR で行う。
    #[error("OAuth token refresh is not implemented yet")]
    NotImplemented,
}

impl AuthError {
    pub fn requires_reauth(&self) -> bool {
        matches!(self, Self::RefreshTokenExpired | Self::Unauthorized)
    }
}

impl Error {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Auth(e) if e.requires_reauth() => EXIT_AUTH_EXPIRED,
            _ => EXIT_FAILURE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes() {
        assert_eq!(Error::Auth(AuthError::RefreshTokenExpired).exit_code(), 2);
        assert_eq!(Error::Auth(AuthError::Unauthorized).exit_code(), 2);
        assert_eq!(Error::Auth(AuthError::NotImplemented).exit_code(), 1);
        assert_eq!(Error::Config("x".into()).exit_code(), 1);
        assert_eq!(
            Error::Api {
                status: 500,
                body: String::new()
            }
            .exit_code(),
            1
        );
    }
}
