use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::test::{Mode, TestResult};

const BASE: &str = "https://api.monkeytype.com";

#[derive(Debug, Deserialize)]
struct ApiEnvelope {
    message: Option<String>,
    data: Option<Value>,
}

/// POST /results — submit a finished test.
///
/// WARNING: Schema is unstable and subject to change. Monkeytype's anti-cheat
/// rejects results missing plausible chartData / keySpacing / keyDuration. This
/// implementation supplies the minimum viable payload — expect 4xx until
/// fields are tuned against the live backend. See backend/src/api/schemas/result-schemas.ts
/// in the monkeytype repo.
pub async fn submit_result(ape_key: &str, r: &TestResult) -> Result<String> {
    let (mode, mode2) = match r.mode {
        Mode::Time(t) => ("time", t.to_string()),
        Mode::Words(n) => ("words", n.to_string()),
    };

    let payload = json!({
        "result": {
            "wpm": r.wpm,
            "rawWpm": r.raw_wpm,
            "acc": r.accuracy,
            "consistency": r.consistency,
            "charStats": [r.correct_chars, r.incorrect_chars, r.extra_chars, r.missed_chars],
            "mode": mode,
            "mode2": mode2,
            "quoteLength": -1,
            "timestamp": chrono_now_ms(),
            "testDuration": r.test_duration,
            "afkDuration": 0.0,
            "language": r.language,
            "punctuation": false,
            "numbers": false,
            "lazyMode": false,
            "blindMode": false,
            "difficulty": "normal",
            "funbox": "none",
            "tags": [],
            "restartCount": 0,
            "incompleteTestSeconds": 0.0,
            "incompleteTests": [],
            "stopOnLetter": false,
            "customText": null,
            "chartData": {
                "wpm": r.wpm_samples,
                "raw": r.raw_samples,
                "err": r.err_samples,
            },
            "keySpacingStats": {"average": 0.0, "sd": 0.0},
            "keyDurationStats": {"average": 0.0, "sd": 0.0},
            "isPb": false,
            "bailedOut": false,
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{BASE}/results"))
        .header("Authorization", format!("ApeKey {ape_key}"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();
    let body: ApiEnvelope = resp.json().await.unwrap_or(ApiEnvelope {
        message: None,
        data: None,
    });

    if !status.is_success() {
        return Err(anyhow!(
            "monkeytype rejected result: {} ({})",
            body.message.unwrap_or_else(|| "no message".into()),
            status
        ));
    }
    let id = body
        .data
        .as_ref()
        .and_then(|d| d.get("insertedId"))
        .and_then(|v| v.as_str())
        .unwrap_or("ok")
        .to_string();
    Ok(id)
}

fn chrono_now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
