use regex::Regex;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::*;

/// 验证邮箱格式
pub fn validate_email_format(email: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    re.is_match(email)
}

/// 验证邮箱域名是否存在MX记录
pub async fn validate_email_mx(email: &str) -> bool {
    let domain = match email.split('@').last() {
        Some(d) => d,
        None => return false,
    };

    let resolver = TokioAsyncResolver::tokio(
        ResolverConfig::default(),
        ResolverOpts::default(),
    );

    match resolver.mx_lookup(domain).await {
        Ok(mx) => mx.iter().next().is_some(),
        Err(_) => false,
    }
}

/// 完整邮箱验证
pub async fn validate_email(email: &str) -> Result<(), &'static str> {
    if !validate_email_format(email) {
        return Err("邮箱格式不正确");
    }
    
    if !validate_email_mx(email).await {
        return Err("邮箱域名无效");
    }
    
    Ok(())
}

/// Markdown 转 HTML
pub fn markdown_to_html(markdown: &str) -> String {
    use pulldown_cmark::{Parser, Options, html};
    
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    html_output
}

/// 生成随机token
pub fn generate_token() -> String {
    use uuid::Uuid;
    format!("{}{}", Uuid::new_v4(), Uuid::new_v4()).replace("-", "")
}
