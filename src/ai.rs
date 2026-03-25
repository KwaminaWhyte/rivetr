//! Multi-provider AI client supporting Claude, OpenAI, Gemini, and Moonshot.
//! All AI features are optional -- if no API key is configured the client is
//! unavailable and callers gracefully skip AI-powered responses.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    Claude,
    OpenAi,
    Gemini,
    Moonshot,
}

impl std::str::FromStr for AiProvider {
    type Err = ();
    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(AiProvider::OpenAi),
            "gemini" => Ok(AiProvider::Gemini),
            "moonshot" => Ok(AiProvider::Moonshot),
            _ => Ok(AiProvider::Claude),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AiClient {
    provider: AiProvider,
    api_key: String,
    model: String,
    max_tokens: u32,
    http: reqwest::Client,
}

impl AiClient {
    pub fn new(
        provider: AiProvider,
        api_key: String,
        model: Option<String>,
        max_tokens: Option<u32>,
    ) -> Self {
        let default_model = match &provider {
            AiProvider::Claude => "claude-sonnet-4-6",
            AiProvider::OpenAi => "gpt-4o",
            AiProvider::Gemini => "gemini-2.0-flash",
            AiProvider::Moonshot => "moonshot-v1-8k",
        };
        Self {
            provider,
            api_key,
            model: model.unwrap_or_else(|| default_model.to_string()),
            max_tokens: max_tokens.unwrap_or(2048),
            http: reqwest::Client::new(),
        }
    }

    /// Send a system+user prompt and return the assistant text response.
    pub async fn complete(&self, system: &str, user: &str) -> Result<String> {
        match &self.provider {
            AiProvider::Claude => self.complete_claude(system, user).await,
            AiProvider::OpenAi => {
                self.complete_openai_compat(
                    "https://api.openai.com/v1/chat/completions",
                    system,
                    user,
                )
                .await
            }
            AiProvider::Moonshot => {
                self.complete_openai_compat(
                    "https://api.moonshot.cn/v1/chat/completions",
                    system,
                    user,
                )
                .await
            }
            AiProvider::Gemini => self.complete_gemini(system, user).await,
        }
    }

    async fn complete_claude(&self, system: &str, user: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Req<'a> {
            model: &'a str,
            max_tokens: u32,
            system: &'a str,
            messages: Vec<Msg<'a>>,
        }
        #[derive(Serialize)]
        struct Msg<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            content: Vec<Block>,
        }
        #[derive(Deserialize)]
        struct Block {
            text: String,
        }

        let body = Req {
            model: &self.model,
            max_tokens: self.max_tokens,
            system,
            messages: vec![Msg {
                role: "user",
                content: user,
            }],
        };
        let resp: Resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .context("Claude request failed")?
            .error_for_status()
            .context("Claude error status")?
            .json()
            .await
            .context("Claude parse failed")?;
        Ok(resp
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join(""))
    }

    async fn complete_openai_compat(&self, url: &str, system: &str, user: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Req<'a> {
            model: &'a str,
            max_tokens: u32,
            messages: Vec<Msg<'a>>,
        }
        #[derive(Serialize)]
        struct Msg<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            choices: Vec<Choice>,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: MsgResp,
        }
        #[derive(Deserialize)]
        struct MsgResp {
            content: String,
        }

        let body = Req {
            model: &self.model,
            max_tokens: self.max_tokens,
            messages: vec![
                Msg {
                    role: "system",
                    content: system,
                },
                Msg {
                    role: "user",
                    content: user,
                },
            ],
        };
        let resp: Resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("OpenAI-compat request failed")?
            .error_for_status()
            .context("OpenAI-compat error status")?
            .json()
            .await
            .context("OpenAI-compat parse failed")?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .context("No choices in response")
    }

    async fn complete_gemini(&self, system: &str, user: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Req<'a> {
            system_instruction: SysInstr<'a>,
            contents: Vec<Content<'a>>,
            #[serde(rename = "generationConfig")]
            generation_config: GenConfig,
        }
        #[derive(Serialize)]
        struct SysInstr<'a> {
            parts: Vec<Part<'a>>,
        }
        #[derive(Serialize)]
        struct Content<'a> {
            role: &'a str,
            parts: Vec<Part<'a>>,
        }
        #[derive(Serialize)]
        struct Part<'a> {
            text: &'a str,
        }
        #[derive(Serialize)]
        struct GenConfig {
            #[serde(rename = "maxOutputTokens")]
            max_output_tokens: u32,
        }
        #[derive(Deserialize)]
        struct Resp {
            candidates: Vec<Candidate>,
        }
        #[derive(Deserialize)]
        struct Candidate {
            content: ContentResp,
        }
        #[derive(Deserialize)]
        struct ContentResp {
            parts: Vec<PartResp>,
        }
        #[derive(Deserialize)]
        struct PartResp {
            text: String,
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );
        let body = Req {
            system_instruction: SysInstr {
                parts: vec![Part { text: system }],
            },
            contents: vec![Content {
                role: "user",
                parts: vec![Part { text: user }],
            }],
            generation_config: GenConfig {
                max_output_tokens: self.max_tokens,
            },
        };
        let resp: Resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Gemini request failed")?
            .error_for_status()
            .context("Gemini error status")?
            .json()
            .await
            .context("Gemini parse failed")?;
        resp.candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .context("No content in Gemini response")
    }
}
