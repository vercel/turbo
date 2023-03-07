use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use turbo_tasks::trace::TraceRawVcs;

/// The top-most structure encoded into the query param in requests to
/// `next/font/local` generated by the next/font swc transform. e.g.
/// `next/font/local/target.css?{"path": "index.js", "src": "./Inter.ttf"...`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NextFontLocalRequest {
    pub path: String,
    pub import: String,
    pub arguments: (NextFontLocalRequestArguments,),
    pub variable_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NextFontLocalRequestArguments {
    pub src: SrcRequest,
    pub weight: Option<String>,
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default = "default_display")]
    pub display: String,
    #[serde(default = "default_preload")]
    pub preload: bool,
    pub fallback: Option<Vec<String>>,
    #[serde(
        default = "default_adjust_font_fallback",
        deserialize_with = "deserialize_adjust_font_fallback"
    )]
    pub adjust_font_fallback: AdjustFontFallback,
    pub variable: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum SrcRequest {
    One(String),
    Many(Vec<SrcDescriptor>),
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SrcDescriptor {
    pub path: String,
    pub weight: Option<String>,
    pub style: Option<String>,
}

#[derive(
    Clone, Debug, Deserialize, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, TraceRawVcs,
)]
pub(crate) enum AdjustFontFallback {
    Arial,
    TimesNewRoman,
    None,
}

fn default_adjust_font_fallback() -> AdjustFontFallback {
    AdjustFontFallback::TimesNewRoman
}

fn deserialize_adjust_font_fallback<'de, D>(
    de: D,
) -> std::result::Result<AdjustFontFallback, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AdjustFontFallbackInner {
        Named(String),
        None(bool),
    }

    match AdjustFontFallbackInner::deserialize(de)? {
        AdjustFontFallbackInner::Named(name) => match name.as_str() {
            "Arial" => Ok(AdjustFontFallback::Arial),
            "Times New Roman" => Ok(AdjustFontFallback::TimesNewRoman),
            _ => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Other("adjust_font_fallback"),
                &"Expected either \"Arial\" or \"Times New Roman\"",
            )),
        },
        AdjustFontFallbackInner::None(val) => {
            if val {
                Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Other("adjust_font_fallback"),
                    &"Expected string or `false`. Received `true`",
                ))
            } else {
                Ok(AdjustFontFallback::None)
            }
        }
    }
}

fn default_preload() -> bool {
    true
}

fn default_display() -> String {
    "swap".to_owned()
}

fn default_style() -> String {
    "normal".to_owned()
}
