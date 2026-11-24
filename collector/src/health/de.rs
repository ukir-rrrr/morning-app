//! 数値が JSON 文字列で返るフィールド（proto3 JSON の int64 など）を受けるためのデシリアライザ。
//!
//! `"217"` / `217` / `217.0` / `null` のいずれも受け付ける。空文字は `None` として扱う。

use serde::{de, Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(untagged)]
enum NumOrStr {
    Int(i64),
    Float(f64),
    Str(String),
}

impl NumOrStr {
    fn into_i64<E: de::Error>(self) -> Result<Option<i64>, E> {
        match self {
            Self::Int(v) => Ok(Some(v)),
            Self::Float(v) if v.fract() == 0.0 && v.is_finite() => Ok(Some(v as i64)),
            Self::Float(v) => Err(E::custom(format!("expected integer, got {v}"))),
            Self::Str(s) => {
                let s = s.trim();
                if s.is_empty() {
                    return Ok(None);
                }
                s.parse::<i64>()
                    .map(Some)
                    .map_err(|e| E::custom(format!("invalid integer string {s:?}: {e}")))
            }
        }
    }

    fn into_f64<E: de::Error>(self) -> Result<Option<f64>, E> {
        match self {
            Self::Int(v) => Ok(Some(v as f64)),
            Self::Float(v) => Ok(Some(v)),
            Self::Str(s) => {
                let s = s.trim();
                if s.is_empty() {
                    return Ok(None);
                }
                s.parse::<f64>()
                    .map(Some)
                    .map_err(|e| E::custom(format!("invalid number string {s:?}: {e}")))
            }
        }
    }
}

pub fn opt_i64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    match Option::<NumOrStr>::deserialize(d)? {
        Some(v) => v.into_i64(),
        None => Ok(None),
    }
}

pub fn opt_f64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    match Option::<NumOrStr>::deserialize(d)? {
        Some(v) => v.into_f64(),
        None => Ok(None),
    }
}

pub fn i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    opt_i64(d)?.ok_or_else(|| de::Error::custom("expected a number"))
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct T {
        #[serde(default, deserialize_with = "super::opt_i64")]
        i: Option<i64>,
        #[serde(default, deserialize_with = "super::opt_f64")]
        f: Option<f64>,
    }

    fn parse(json: &str) -> T {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn accepts_strings_numbers_and_null() {
        let t = parse(r#"{"i":"217","f":"95.5"}"#);
        assert_eq!((t.i, t.f), (Some(217), Some(95.5)));
        let t = parse(r#"{"i":217,"f":95}"#);
        assert_eq!((t.i, t.f), (Some(217), Some(95.0)));
        let t = parse(r#"{"i":null,"f":""}"#);
        assert_eq!((t.i, t.f), (None, None));
        let t = parse(r#"{}"#);
        assert_eq!((t.i, t.f), (None, None));
    }

    #[test]
    fn rejects_garbage() {
        assert!(serde_json::from_str::<T>(r#"{"i":"abc"}"#).is_err());
        assert!(serde_json::from_str::<T>(r#"{"i":1.5}"#).is_err());
    }
}
