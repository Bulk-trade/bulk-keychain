//! Exact SDK clear-sign text for the explicitly supported wallet action subset.
use crate::{Error, PreparedMessage, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub(crate) fn canonical_message(prepared: &PreparedMessage) -> Result<String> {
    let network = match prepared.message_bytes.last() {
        Some(1) => "mainnet",
        Some(2) => "testnet",
        Some(3) => "devnet",
        _ => return Err(invalid("network domain")),
    };
    let mut message = format!(
        "Bulk Exchange Transaction\nNetwork: {network}\nAccount: {}\nNonce: {}\nActions: {}\nSignable-Hash: {}\n",
        prepared.account, prepared.nonce, prepared.actions.len(), hex::encode(Sha256::digest(&prepared.message_bytes))
    );
    for (index, action) in prepared.actions.iter().enumerate() {
        writeln!(message, "[{index}] {}", action_line(action, 0)?).expect("String write");
    }
    Ok(message)
}

fn invalid(field: &str) -> Error {
    Error::SigningFailed(format!("invalid offchain clear-sign field: {field}"))
}
fn text<'a>(body: &'a Value, key: &str) -> Result<&'a str> {
    body[key].as_str().ok_or_else(|| invalid(key))
}
fn boolean(body: &Value, key: &str) -> Result<bool> {
    body[key].as_bool().ok_or_else(|| invalid(key))
}
fn number(value: &Value) -> Result<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|value| value.is_finite())
        .ok_or_else(|| invalid("finite number"))
}
fn integer(body: &Value, key: &str) -> Result<u64> {
    body[key].as_u64().ok_or_else(|| invalid(key))
}
// Mirrors SDK canonicalize_safe_f64, including the stable fixed-point ULP search.
fn safe(value: &Value) -> Result<String> {
    let value = number(value)?;
    let fixed = value * 1e8;
    if value > 0.0 && fixed.is_finite() && fixed < u64::MAX as f64 {
        let rounded = fixed.round() as u64;
        if rounded != 0 && rounded != u64::MAX {
            let mut bits = (rounded as f64 / 1e8).to_bits();
            for _ in 0..16 {
                let candidate = f64::from_bits(bits);
                if !candidate.is_finite() || candidate <= 0.0 {
                    break;
                }
                let back = (candidate * 1e8).round() as u64;
                if back == rounded {
                    return Ok(format!("{candidate:.8}"));
                }
                bits = if back > rounded {
                    bits.wrapping_sub(1)
                } else {
                    bits.wrapping_add(1)
                };
            }
            return Ok(format!("{:.8}", rounded as f64 / 1e8));
        }
    }
    Ok(format!("{value:.8}"))
}
fn optional(value: &Value, fixed: bool) -> Result<String> {
    if value.is_null() {
        Ok("-".into())
    } else if fixed {
        safe(value)
    } else {
        Ok(format!("{:.8}", number(value)?))
    }
}
fn optional_slippage(body: &Value, key: &str, label: &str) -> Result<String> {
    match body.get(key) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(value) => Ok(format!(" {label}={}bps", safe(value)?)),
    }
}

fn action_line(action: &Value, depth: usize) -> Result<String> {
    if depth > 32 {
        return Err(invalid("action nesting"));
    }
    let object = action
        .as_object()
        .filter(|object| object.len() == 1)
        .ok_or_else(|| invalid("action tag"))?;
    let (tag, body) = object.iter().next().ok_or_else(|| invalid("action tag"))?;
    if !body.is_object() {
        return Err(invalid("action body"));
    }
    match tag.as_str() {
        "m" => Ok(format!(
            "Market {} {} sz={} ro={} iso={}{}",
            text(body, "c")?,
            if boolean(body, "b")? { "Buy" } else { "Sell" },
            safe(&body["sz"])?,
            boolean(body, "r")?,
            body.get("i")
                .map(|_| boolean(body, "i"))
                .transpose()?
                .unwrap_or(false),
            if body["slippage"].is_null() {
                String::new()
            } else {
                format!(" slippage={}bps", safe(&body["slippage"])?)
            }
        )),
        "l" => {
            let tif = text(body, "tif")?;
            if !matches!(tif, "GTC" | "IOC" | "ALO") {
                return Err(invalid("tif"));
            }
            Ok(format!(
                "Limit {} {} px={} sz={} tif={} ro={} iso={}",
                text(body, "c")?,
                if boolean(body, "b")? { "Buy" } else { "Sell" },
                safe(&body["px"])?,
                safe(&body["sz"])?,
                tif,
                boolean(body, "r")?,
                body.get("i")
                    .map(|_| boolean(body, "i"))
                    .transpose()?
                    .unwrap_or(false)
            ))
        }
        "mod" => Ok(format!(
            "Modify {} oid={} sz={}",
            text(body, "c")?,
            text(body, "oid")?,
            safe(&body["sz"])?
        )),
        "cx" => Ok(format!(
            "Cancel {} oid={}",
            text(body, "c")?,
            text(body, "oid")?
        )),
        "cxa" => {
            let symbols = body["c"]
                .as_array()
                .ok_or_else(|| invalid("c"))?
                .iter()
                .map(|v| v.as_str().ok_or_else(|| invalid("symbol")))
                .collect::<Result<Vec<_>>>()?;
            Ok(format!(
                "CancelAll {}",
                if symbols.is_empty() {
                    "*".into()
                } else {
                    symbols.join(",")
                }
            ))
        }
        "st" | "tp" => Ok(format!(
            "{} {} {} thresh={} sz={} limit={}{}",
            if tag == "st" { "Stop" } else { "TakeProfit" },
            text(body, "c")?,
            if boolean(body, "d")? {
                "Above"
            } else {
                "Below"
            },
            safe(&body["tr"])?,
            safe(&body["sz"])?,
            optional(&body["lim"], true)?,
            optional_slippage(body, "slippage", "slippage")?
        )),
        "rng" => Ok(format!(
            "Range {} {} min={} max={} sz={} lmin={} lmax={}{}{}",
            text(body, "c")?,
            if boolean(body, "d")? { "Buy" } else { "Sell" },
            safe(&body["pmin"])?,
            safe(&body["pmax"])?,
            safe(&body["sz"])?,
            optional(&body["lmin"], true)?,
            optional(&body["lmax"], true)?,
            optional_slippage(body, "slSlippage", "sl_slippage")?,
            optional_slippage(body, "tpSlippage", "tp_slippage")?
        )),
        "trig" => {
            let actions = body["actions"]
                .as_array()
                .ok_or_else(|| invalid("actions"))?;
            for action in actions {
                action_line(action, depth + 1)?;
            }
            Ok(format!(
                "Trigger {} {} thresh={} nested={}",
                text(body, "c")?,
                if boolean(body, "d")? {
                    "Above"
                } else {
                    "Below"
                },
                safe(&body["tr"])?,
                actions.len()
            ))
        }
        "trl" => Ok(format!(
            "Trailing {} {} sz={} trail={}bps step={}bps limit={}{}",
            text(body, "c")?,
            if boolean(body, "b")? { "Buy" } else { "Sell" },
            safe(&body["sz"])?,
            integer(body, "trb")?,
            integer(body, "stb")?,
            optional(&body["lim"], true)?,
            optional_slippage(body, "slippage", "slippage")?
        )),
        "faucet" => Ok(format!(
            "Faucet user={} amount={}",
            text(body, "u")?,
            optional(&body["amount"], false)?
        )),
        "agentWalletCreation" => Ok(format!(
            "AgentWallet agent={} delete={}",
            text(body, "a")?,
            boolean(body, "d")?
        )),
        "updateUserSettings" => {
            let mut pairs = body["m"]
                .as_object()
                .ok_or_else(|| invalid("m"))?
                .iter()
                .collect::<Vec<_>>();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            Ok(format!(
                "UpdateLeverage {}",
                pairs
                    .into_iter()
                    .map(|(symbol, value)| Ok(format!("{symbol}:{:.8}", number(value)?)))
                    .collect::<Result<Vec<_>>>()?
                    .join(",")
            ))
        }
        "createSubAccount" => Ok(format!(
            "CreateSubAccount name={} amt={}",
            text(body, "name")?,
            optional(&body["marginAmount"], false)?
        )),
        "removeSubAccount" => Ok(format!("RemoveSubAccount {}", text(body, "toRemove")?)),
        "renameSubAccount" | "rsa" => Ok(format!(
            "RenameSubAccount account={} name={}",
            text(body, "account")?,
            text(body, "name")?
        )),
        "transfer" => Ok(format!(
            "Transfer {} from={} to={} amt={:.8}",
            match text(body, "k")? {
                "internal" => "Internal",
                "external" => "External",
                _ => return Err(invalid("transfer kind")),
            },
            text(body, "from")?,
            text(body, "to")?,
            number(&body["marginAmount"])?
        )),
        "abc" => Ok(format!(
            "ApproveCommissionFee to={} max_fee={}",
            text(body, "to")?,
            integer(body, "fee")?
        )),
        "rbc" => Ok(format!("RevokeCommissionFee to={}", text(body, "to")?)),
        "whitelistFaucet" => Ok(format!(
            "WhitelistFaucet target={} whitelist={}",
            text(body, "target")?,
            boolean(body, "whitelist")?
        )),
        _ => Err(Error::SigningFailed(format!(
            "unsupported offchain clear-sign action: {tag}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_actions_match_pinned_sdk_clear_text() {
        let fixtures: Value =
            serde_json::from_str(include_str!("../tests/fixtures/clear_sign_sdk.json")).unwrap();
        for vector in fixtures["vectors"].as_array().unwrap() {
            let prepared = PreparedMessage {
                message_bytes: hex::decode(vector["message_hex"].as_str().unwrap()).unwrap(),
                actions: serde_json::from_value(vector["actions"].clone()).unwrap(),
                account: "11111111111111111111111111111111".into(),
                signer: "11111111111111111111111111111111".into(),
                nonce: 42,
                order_id: None,
                order_ids: None,
            };
            assert_eq!(
                canonical_message(&prepared).unwrap(),
                vector["canonical_text"].as_str().unwrap(),
                "{}",
                vector["actions"]
            );
        }
    }

    #[test]
    fn rejects_unverified_actions_and_nested_on_fill() {
        for action in [
            serde_json::json!({"of": {}}),
            serde_json::json!({"msp": {}}),
            serde_json::json!({"liq": {}}),
            serde_json::json!({"msu": {}}),
            serde_json::json!({"unknown": {}}),
            serde_json::json!({"trig": {"c":"BTC-USD", "d":true, "tr":100, "actions":[{"of":{}}]}}),
            serde_json::json!({"m": {"c":"BTC-USD", "b":true, "sz":"NaN", "r":false}}),
        ] {
            assert!(matches!(
                action_line(&action, 0),
                Err(Error::SigningFailed(_))
            ));
        }
    }
}
