//! 設定は環境変数からのみ読み込む（ファイルからの読み込みは行わない）。

use std::fmt;

use crate::{Error, Result};

pub const ENV_GOOGLE_CLIENT_ID: &str = "GOOGLE_CLIENT_ID";
pub const ENV_GOOGLE_CLIENT_SECRET: &str = "GOOGLE_CLIENT_SECRET";
pub const ENV_GOOGLE_REFRESH_TOKEN: &str = "GOOGLE_REFRESH_TOKEN";
pub const ENV_TURSO_DATABASE_URL: &str = "TURSO_DATABASE_URL";
pub const ENV_TURSO_AUTH_TOKEN: &str = "TURSO_AUTH_TOKEN";

/// `TURSO_DATABASE_URL` 未設定時に使うローカル開発用 DB ファイル。
pub const DEFAULT_LOCAL_DB_PATH: &str = "morning-local.db";

/// Debug / Display で値を出さない秘密値。
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseTarget {
    /// Turso / sqld などのリモート libSQL。
    Remote { url: String, auth_token: Secret },
    /// ローカルファイル DB（開発用フォールバック）。
    Local { path: String },
}

#[derive(Debug, Clone)]
pub struct GoogleCredentials {
    pub client_id: String,
    pub client_secret: Secret,
    pub refresh_token: Secret,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database: DatabaseTarget,
    google_client_id: Option<String>,
    google_client_secret: Option<Secret>,
    google_refresh_token: Option<Secret>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// テスト用に任意のルックアップ関数から構築する。空文字は未設定扱い。
    pub fn from_lookup<F>(lookup: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let get = |key: &str| lookup(key).filter(|v| !v.trim().is_empty());

        let database = match get(ENV_TURSO_DATABASE_URL) {
            None => DatabaseTarget::Local {
                path: DEFAULT_LOCAL_DB_PATH.to_string(),
            },
            Some(url) => match url.strip_prefix("file:") {
                Some(path) => DatabaseTarget::Local {
                    path: path.to_string(),
                },
                None => {
                    let auth_token = get(ENV_TURSO_AUTH_TOKEN).ok_or_else(|| {
                        Error::Config(format!(
                            "{ENV_TURSO_AUTH_TOKEN} is required when {ENV_TURSO_DATABASE_URL} is a remote URL"
                        ))
                    })?;
                    DatabaseTarget::Remote {
                        url,
                        auth_token: Secret::new(auth_token),
                    }
                }
            },
        };

        Ok(Self {
            database,
            google_client_id: get(ENV_GOOGLE_CLIENT_ID),
            google_client_secret: get(ENV_GOOGLE_CLIENT_SECRET).map(Secret::new),
            google_refresh_token: get(ENV_GOOGLE_REFRESH_TOKEN).map(Secret::new),
        })
    }

    /// Google OAuth の資格情報。欠けている環境変数があればまとめてエラーにする。
    pub fn google_credentials(&self) -> Result<GoogleCredentials> {
        let mut missing = Vec::new();
        if self.google_client_id.is_none() {
            missing.push(ENV_GOOGLE_CLIENT_ID);
        }
        if self.google_client_secret.is_none() {
            missing.push(ENV_GOOGLE_CLIENT_SECRET);
        }
        if self.google_refresh_token.is_none() {
            missing.push(ENV_GOOGLE_REFRESH_TOKEN);
        }
        match (
            &self.google_client_id,
            &self.google_client_secret,
            &self.google_refresh_token,
        ) {
            (Some(id), Some(secret), Some(refresh)) => Ok(GoogleCredentials {
                client_id: id.clone(),
                client_secret: secret.clone(),
                refresh_token: refresh.clone(),
            }),
            _ => Err(Error::Config(format!(
                "missing environment variables: {}",
                missing.join(", ")
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config(vars: &[(&str, &str)]) -> Result<Config> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn falls_back_to_local_db() {
        let c = config(&[]).unwrap();
        assert_eq!(
            c.database,
            DatabaseTarget::Local {
                path: DEFAULT_LOCAL_DB_PATH.into()
            }
        );
        let c = config(&[(ENV_TURSO_DATABASE_URL, "file:/tmp/dev.db")]).unwrap();
        assert_eq!(
            c.database,
            DatabaseTarget::Local {
                path: "/tmp/dev.db".into()
            }
        );
    }

    #[test]
    fn remote_requires_auth_token() {
        assert!(config(&[(ENV_TURSO_DATABASE_URL, "libsql://example.turso.io")]).is_err());
        let c = config(&[
            (ENV_TURSO_DATABASE_URL, "libsql://example.turso.io"),
            (ENV_TURSO_AUTH_TOKEN, "dummy"),
        ])
        .unwrap();
        assert!(matches!(c.database, DatabaseTarget::Remote { .. }));
    }

    #[test]
    fn google_credentials_reports_missing_and_redacts() {
        let err = config(&[(ENV_GOOGLE_CLIENT_ID, "id")])
            .unwrap()
            .google_credentials()
            .unwrap_err()
            .to_string();
        assert!(err.contains(ENV_GOOGLE_CLIENT_SECRET));
        assert!(err.contains(ENV_GOOGLE_REFRESH_TOKEN));

        let creds = config(&[
            (ENV_GOOGLE_CLIENT_ID, "id"),
            (ENV_GOOGLE_CLIENT_SECRET, "dummy-secret"),
            (ENV_GOOGLE_REFRESH_TOKEN, "dummy-refresh"),
        ])
        .unwrap()
        .google_credentials()
        .unwrap();
        let debug = format!("{creds:?}");
        assert!(!debug.contains("dummy-secret"));
        assert!(!debug.contains("dummy-refresh"));
    }
}
