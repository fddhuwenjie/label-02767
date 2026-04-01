use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::env;
use chrono::{Utc, DateTime, TimeZone};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct PaymentConfig {
    pub alipay_app_id: String,
    pub alipay_private_key: String,
    pub alipay_public_key: String,
    pub alipay_notify_url: String,
    pub alipay_return_url: String,
    pub alipay_gateway: String,

    pub wechat_app_id: String,
    pub wechat_mch_id: String,
    pub wechat_api_key: String,
    pub wechat_notify_url: String,
}

impl PaymentConfig {
    pub fn from_env() -> Self {
        let base_url = env::var("PAYMENT_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8090".to_string());

        Self {
            alipay_app_id: env::var("ALIPAY_APP_ID").unwrap_or_default(),
            alipay_private_key: env::var("ALIPAY_PRIVATE_KEY").unwrap_or_default(),
            alipay_public_key: env::var("ALIPAY_PUBLIC_KEY").unwrap_or_default(),
            alipay_notify_url: env::var("ALIPAY_NOTIFY_URL")
                .unwrap_or_else(|_| format!("{}/api/payment/callback/alipay", base_url)),
            alipay_return_url: env::var("ALIPAY_RETURN_URL")
                .unwrap_or_else(|_| format!("{}/profile", base_url)),
            alipay_gateway: env::var("ALIPAY_GATEWAY")
                .unwrap_or_else(|_| "https://openapi.alipay.com/gateway.do".to_string()),

            wechat_app_id: env::var("WECHAT_APP_ID").unwrap_or_default(),
            wechat_mch_id: env::var("WECHAT_MCH_ID").unwrap_or_default(),
            wechat_api_key: env::var("WECHAT_API_KEY").unwrap_or_default(),
            wechat_notify_url: env::var("WECHAT_NOTIFY_URL")
                .unwrap_or_else(|_| format!("{}/api/payment/callback/wechat", base_url)),
        }
    }

    pub fn alipay_configured(&self) -> bool {
        !self.alipay_app_id.is_empty() && !self.alipay_private_key.is_empty()
    }

    pub fn wechat_configured(&self) -> bool {
        !self.wechat_app_id.is_empty() && !self.wechat_mch_id.is_empty() && !self.wechat_api_key.is_empty()
    }
}

pub fn generate_alipay_form(
    config: &PaymentConfig,
    order_id: &str,
    amount: f64,
    subject: &str,
) -> String {
    let timestamp = chrono::Utc::now()
        .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let biz_content = serde_json::json!({
        "out_trade_no": order_id,
        "total_amount": format!("{:.2}", amount),
        "subject": subject,
        "product_code": "FAST_INSTANT_TRADE_PAY"
    });

    let mut params = BTreeMap::new();
    params.insert("app_id", config.alipay_app_id.clone());
    params.insert("method", "alipay.trade.page.pay".to_string());
    params.insert("charset", "utf-8".to_string());
    params.insert("sign_type", "RSA2".to_string());
    params.insert("timestamp", timestamp);
    params.insert("version", "1.0".to_string());
    params.insert("notify_url", config.alipay_notify_url.clone());
    params.insert("return_url", config.alipay_return_url.clone());
    params.insert("biz_content", biz_content.to_string());

    let sign_str: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let signature = sign_with_rsa2(&config.alipay_private_key, &sign_str);

    let encoded_params: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect();

    format!(
        "{}?{}&sign={}",
        config.alipay_gateway,
        encoded_params.join("&"),
        urlencoding::encode(&signature)
    )
}

fn sign_with_rsa2(private_key: &str, content: &str) -> String {
    use ::hmac::Mac;
    let key = if private_key.is_empty() {
        "default_dev_key".as_bytes()
    } else {
        private_key.as_bytes()
    };
    let mut mac = HmacSha256::new_from_slice(key)
        .expect("HMAC key error");
    mac.update(content.as_bytes());
    let result = mac.finalize();
    base64_encode(&result.into_bytes())
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

pub fn generate_wechat_pay_params(
    config: &PaymentConfig,
    order_id: &str,
    amount: f64,
    description: &str,
    client_ip: &str,
) -> BTreeMap<String, String> {
    let nonce = uuid::Uuid::new_v4().to_string().replace("-", "");
    let amount_fen = (amount * 100.0) as i64;

    let mut params = BTreeMap::new();
    params.insert("appid".to_string(), config.wechat_app_id.clone());
    params.insert("mch_id".to_string(), config.wechat_mch_id.clone());
    params.insert("nonce_str".to_string(), nonce);
    params.insert("body".to_string(), description.to_string());
    params.insert("out_trade_no".to_string(), order_id.to_string());
    params.insert("total_fee".to_string(), amount_fen.to_string());
    params.insert("spbill_create_ip".to_string(), client_ip.to_string());
    params.insert("notify_url".to_string(), config.wechat_notify_url.clone());
    params.insert("trade_type".to_string(), "NATIVE".to_string());

    let sign = generate_wechat_sign(&params, &config.wechat_api_key);
    params.insert("sign".to_string(), sign);

    params
}

pub fn generate_wechat_sign(params: &BTreeMap<String, String>, api_key: &str) -> String {
    let sign_str: String = params
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let sign_str = format!("{}&key={}", sign_str, api_key);

    let digest = <sha2::Sha256 as sha2::Digest>::digest(sign_str.as_bytes());
    hex_encode(&digest).to_uppercase()
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn params_to_xml(params: &BTreeMap<String, String>) -> String {
    let mut xml = String::from("<xml>");
    for (k, v) in params {
        xml.push_str(&format!("<{}>{}</{}>", k, v, k));
    }
    xml.push_str("</xml>");
    xml
}

pub fn verify_alipay_callback(params: &BTreeMap<String, String>, public_key: &str) -> bool {
    let sign = match params.get("sign") {
        Some(s) => s.clone(),
        None => return false,
    };

    let mut verify_params = params.clone();
    verify_params.remove("sign");
    verify_params.remove("sign_type");

    let sign_str: String = verify_params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let expected = sign_with_rsa2(public_key, &sign_str);
    expected == sign
}

pub fn verify_wechat_callback(params: &BTreeMap<String, String>, api_key: &str) -> bool {
    let sign = match params.get("sign") {
        Some(s) => s.clone(),
        None => return false,
    };

    let mut verify_params = params.clone();
    verify_params.remove("sign");

    let expected = generate_wechat_sign(&verify_params, api_key);
    expected == sign
}

pub fn parse_xml_to_map(xml: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut current_tag = String::new();
    let mut current_value = String::new();
    let mut in_tag = false;
    let mut in_value = false;

    for ch in xml.chars() {
        match ch {
            '<' => {
                if in_value && !current_tag.is_empty() {
                    map.insert(current_tag.clone(), current_value.clone());
                    current_value.clear();
                }
                in_tag = true;
                in_value = false;
                current_tag.clear();
            }
            '>' => {
                in_tag = false;
                if !current_tag.starts_with('/') && current_tag != "xml" {
                    in_value = true;
                    current_value.clear();
                }
            }
            _ => {
                if in_tag {
                    current_tag.push(ch);
                } else if in_value {
                    current_value.push(ch);
                }
            }
        }
    }
    map
}

pub fn verify_callback_signature(params: &BTreeMap<String, String>, secret: &str) -> bool {
    let signature = match params.get("signature") {
        Some(s) => s.clone(),
        None => return false,
    };

    let mut verify_params = params.clone();
    verify_params.remove("signature");

    let sign_str: String = verify_params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC key error");
    mac.update(sign_str.as_bytes());
    let result = mac.finalize();
    let expected = hex_encode(&result.into_bytes());

    expected == signature.to_lowercase()
}

pub fn verify_callback_timestamp(params: &BTreeMap<String, String>) -> bool {
    let timestamp_str = match params.get("timestamp") {
        Some(t) => t,
        None => return false,
    };

    let timestamp = match timestamp_str.parse::<i64>() {
        Ok(t) => t,
        Err(_) => return false,
    };

    let request_time = match Utc.timestamp_millis_opt(timestamp) {
        chrono::LocalResult::Single(t) => t,
        _ => return false,
    };

    let now = Utc::now();
    let diff = now - request_time;

    diff.num_minutes() <= 5
}

pub fn get_payment_secret() -> String {
    env::var("PAYMENT_SECRET")
        .unwrap_or_else(|_| "default_payment_secret_key_change_in_production".to_string())
}

mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::new();
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        result
    }
}
