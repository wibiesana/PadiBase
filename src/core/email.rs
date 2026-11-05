//! Email service & SMTP client for PadiBase
//!
//! Handles SMTP configuration, HTML email rendering, and asynchronous
//! email dispatch for user registration, password reset, security notifications,
//! and admin actions.

use anyhow::{anyhow, Result};
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::extension::ClientId;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

use super::db::{get_system_settings, DbPool};
use super::env::Env;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmtpEvents {
    #[serde(default = "default_true")]
    pub welcome: bool,
    #[serde(default = "default_true")]
    pub forgot_password: bool,
    #[serde(default = "default_true")]
    pub password_changed: bool,
    #[serde(default = "default_true")]
    pub admin_created: bool,
    #[serde(default = "default_true")]
    pub verification: bool,
}

fn default_true() -> bool {
    true
}

fn default_auth_method() -> String {
    "LOGIN".to_string()
}

impl Default for SmtpEvents {
    fn default() -> Self {
        Self {
            welcome: true,
            forgot_password: true,
            password_changed: true,
            admin_created: true,
            verification: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailTemplate {
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SmtpTemplates {
    #[serde(default)]
    pub welcome: Option<EmailTemplate>,
    #[serde(default)]
    pub forgot_password: Option<EmailTemplate>,
    #[serde(default)]
    pub password_changed: Option<EmailTemplate>,
    #[serde(default)]
    pub admin_created: Option<EmailTemplate>,
    #[serde(default)]
    pub verification: Option<EmailTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmtpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_from_name")]
    pub from_name: String,
    #[serde(default = "default_from_address")]
    pub from_address: String,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_smtp_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_encryption")]
    pub encryption: String, // "always", "opportunistic", "disabled"
    #[serde(default = "default_auth_method")]
    pub auth_method: String, // "LOGIN", "PLAIN", "NONE"
    #[serde(default)]
    pub ehlo_domain: String, // EHLO/HELO domain
    #[serde(default)]
    pub events: SmtpEvents,
    #[serde(default)]
    pub templates: SmtpTemplates,
}

fn default_smtp_port() -> u16 {
    587
}

fn default_encryption() -> String {
    "opportunistic".to_string()
}

fn default_from_name() -> String {
    "PadiBase Studio".to_string()
}

fn default_from_address() -> String {
    "noreply@example.com".to_string()
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            from_name: "PadiBase Studio".to_string(),
            from_address: "noreply@example.com".to_string(),
            host: String::new(),
            port: 587,
            username: String::new(),
            password: String::new(),
            encryption: "opportunistic".to_string(),
            auth_method: "LOGIN".to_string(),
            ehlo_domain: String::new(),
            events: SmtpEvents::default(),
            templates: SmtpTemplates::default(),
        }
    }
}

/// Retrieve the active SMTP configuration from database settings with environment variable fallbacks.
pub async fn get_smtp_config(pool: &DbPool) -> SmtpConfig {
    let mut config = SmtpConfig::default();

    // Check system settings stored in database
    if let Ok(settings) = get_system_settings(pool).await {
        if let Some(smtp_val) = settings.get("smtp") {
            if let Ok(parsed) = serde_json::from_value::<SmtpConfig>(smtp_val.clone()) {
                config = parsed;
            }
        }
    }

    // Apply environment variables fallback if values are not configured or empty
    if !config.enabled {
        let env_enabled = Env::get_or("SMTP_ENABLED", "false");
        if env_enabled.eq_ignore_ascii_case("true") || env_enabled == "1" {
            config.enabled = true;
        }
    }

    if config.host.is_empty() {
        if let Some(h) = Env::get("SMTP_HOST") {
            config.host = h;
        }
    }

    if config.port == 587 || config.port == 0 {
        if let Some(p_str) = Env::get("SMTP_PORT") {
            if let Ok(p) = p_str.parse::<u16>() {
                config.port = p;
            }
        }
    }

    if config.username.is_empty() {
        if let Some(u) = Env::get("SMTP_USERNAME").or_else(|| Env::get("SMTP_USER")) {
            config.username = u;
        }
    }

    if config.password.is_empty() {
        if let Some(pw) = Env::get("SMTP_PASSWORD").or_else(|| Env::get("SMTP_PASS")) {
            config.password = pw;
        }
    }

    if config.encryption.is_empty() || config.encryption == "starttls" {
        if let Some(enc) = Env::get("SMTP_ENCRYPTION") {
            config.encryption = enc;
        }
    }

    if config.from_name.is_empty() || config.from_name == "PadiBase Studio" {
        if let Some(name) = Env::get("SMTP_FROM_NAME").or_else(|| Env::get("APP_NAME")) {
            config.from_name = name;
        }
    }

    if config.from_address.is_empty() || config.from_address == "noreply@example.com" {
        if let Some(addr) = Env::get("SMTP_FROM_ADDRESS").or_else(|| Env::get("SMTP_FROM")) {
            config.from_address = addr;
        }
    }

    if let Some(am) = Env::get("SMTP_AUTH_METHOD") {
        config.auth_method = am;
    }

    if config.ehlo_domain.is_empty() {
        if let Some(ehlo) = Env::get("SMTP_EHLO_DOMAIN").or_else(|| Env::get("SMTP_HELO_DOMAIN")) {
            config.ehlo_domain = ehlo;
        }
    }

    config
}

/// Send an HTML email using the given SMTP configuration.
pub async fn send_email_with_config(
    config: &SmtpConfig,
    to: &str,
    subject: &str,
    html_body: &str,
) -> Result<()> {
    if !config.enabled {
        info!("Email sending is disabled. Skipped sending email to {}", to);
        return Ok(());
    }

    if config.host.trim().is_empty() {
        warn!("SMTP host is empty. Cannot send email to {}", to);
        return Err(anyhow!("SMTP host is not configured"));
    }

    let from_mailbox: Mailbox = format!("{} <{}>", config.from_name, config.from_address)
        .parse()
        .map_err(|e| anyhow!("Invalid sender mailbox: {}", e))?;

    let to_mailbox: Mailbox = to
        .parse()
        .map_err(|e| anyhow!("Invalid recipient email address: {}", e))?;

    let email = Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body.to_string())
        .map_err(|e| anyhow!("Failed to build email message: {}", e))?;

    let enc = config.encryption.to_lowercase();
    let port = config.port;

    let is_disabled = enc == "disabled" || enc == "none" || enc == "plain";
    let is_tls = enc == "always" || enc == "tls" || enc == "ssl" || port == 465;

    let mut builder = if is_disabled {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            .port(port)
            .timeout(Some(Duration::from_secs(15)))
    } else if is_tls {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|e| anyhow!("Failed to initialize SMTP TLS transport: {}", e))?
            .port(port)
            .timeout(Some(Duration::from_secs(15)))
    } else {
        // Default to STARTTLS / Opportunistic (standard for port 587 or 25)
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
            .map_err(|e| anyhow!("Failed to initialize SMTP STARTTLS transport: {}", e))?
            .port(port)
            .timeout(Some(Duration::from_secs(15)))
    };

    if !config.ehlo_domain.trim().is_empty() {
        builder = builder.hello_name(ClientId::Domain(config.ehlo_domain.trim().to_string()));
    }

    let auth_m = config.auth_method.trim().to_uppercase();
    if auth_m != "NONE" && !config.username.is_empty() && !config.password.is_empty() {
        if auth_m == "PLAIN" {
            builder = builder.authentication(vec![Mechanism::Plain]);
        } else {
            builder = builder.authentication(vec![Mechanism::Login]);
        }
        builder = builder.credentials(Credentials::new(
            config.username.clone(),
            config.password.clone(),
        ));
    }

    let transport = builder.build();

    transport
        .send(email)
        .await
        .map_err(|e| anyhow!("SMTP send error: {}", e))?;

    info!("Email successfully sent to {} with subject: {}", to, subject);
    Ok(())
}

/// Send an email asynchronously using system configuration.
pub async fn send_email(
    pool: &DbPool,
    to: &str,
    subject: &str,
    html_body: &str,
) -> Result<()> {
    let config = get_smtp_config(pool).await;
    send_email_with_config(&config, to, subject, html_body).await
}

/// Fire-and-forget email sending in a background task (won't block caller HTTP requests).
pub fn spawn_send_email(
    pool: DbPool,
    to: String,
    subject: String,
    html_body: String,
) {
    tokio::spawn(async move {
        if let Err(e) = send_email(&pool, &to, &subject, &html_body).await {
            error!("Failed to send background email to {}: {}", to, e);
        }
    });
}

// ============================================================================
// HTML Email Templates
// ============================================================================

/// Wrap email contents in a modern, responsive HTML container.
pub fn render_html_layout(title: &str, body_content: &str, app_name: &str, app_url: &str) -> String {
    let year = chrono::Utc::now().format("%Y");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <style>
    body {{
      margin: 0;
      padding: 0;
      background-color: #f1f5f9;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
      color: #1e293b;
      -webkit-font-smoothing: antialiased;
    }}
    .wrapper {{
      width: 100%;
      background-color: #f1f5f9;
      padding: 40px 15px;
      box-sizing: border-box;
    }}
    .card {{
      max-width: 580px;
      margin: 0 auto;
      background-color: #ffffff;
      border-radius: 16px;
      overflow: hidden;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.05), 0 8px 10px -6px rgba(0, 0, 0, 0.01);
      border: 1px solid #e2e8f0;
    }}
    .header {{
      background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
      padding: 32px 36px;
      text-align: left;
    }}
    .header-logo {{
      display: inline-flex;
      align-items: center;
      gap: 10px;
      color: #ffffff;
      font-size: 20px;
      font-weight: 800;
      text-decoration: none;
      letter-spacing: -0.5px;
    }}
    .header-badge {{
      display: inline-block;
      background-color: #10b981;
      color: #ffffff;
      font-size: 10px;
      font-weight: 700;
      text-transform: uppercase;
      padding: 3px 8px;
      border-radius: 9999px;
      letter-spacing: 0.5px;
      margin-left: 8px;
      vertical-align: middle;
    }}
    .content {{
      padding: 36px;
      line-height: 1.6;
      font-size: 15px;
    }}
    .btn {{
      display: inline-block;
      background-color: #059669;
      color: #ffffff !important;
      text-decoration: none;
      font-size: 14px;
      font-weight: 700;
      padding: 13px 26px;
      border-radius: 10px;
      margin: 20px 0;
      box-shadow: 0 4px 6px -1px rgba(5, 150, 105, 0.2);
    }}
    .token-box {{
      background-color: #f8fafc;
      border: 1px dashed #cbd5e1;
      border-radius: 12px;
      padding: 18px;
      margin: 20px 0;
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-size: 18px;
      font-weight: 700;
      letter-spacing: 1.5px;
      text-align: center;
      color: #0f172a;
      word-break: break-all;
    }}
    .footer {{
      padding: 24px 36px;
      background-color: #f8fafc;
      border-top: 1px solid #f1f5f9;
      text-align: center;
      font-size: 12px;
      color: #64748b;
    }}
    .footer a {{
      color: #059669;
      text-decoration: none;
    }}
  </style>
</head>
<body>
  <div class="wrapper">
    <div class="card">
      <div class="header">
        <a href="{app_url}" class="header-logo">
          <span>{app_name}</span>
          <span class="header-badge">Notification</span>
        </a>
      </div>
      <div class="content">
        {body_content}
      </div>
      <div class="footer">
        <p>Sent by <strong>{app_name}</strong> &bull; <a href="{app_url}">{app_url}</a></p>
        <p style="margin-top: 6px; font-size: 11px;">&copy; {year} {app_name}. All rights reserved.</p>
      </div>
    </div>
  </div>
</body>
</html>"#,
        title = title,
        app_name = app_name,
        app_url = app_url,
        body_content = body_content,
        year = year
    )
}

fn replace_placeholders(text: &str, pairs: &[(&str, &str)]) -> String {
    let mut res = text.to_string();
    for (k, v) in pairs {
        res = res.replace(k, v);
    }
    res
}

pub fn default_welcome_template() -> EmailTemplate {
    EmailTemplate {
        subject: "Welcome to {appName}!".to_string(),
        body: r#"<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Welcome, {name}! 🎉</h2>
<p>Thank you for creating an account with <strong>{appName}</strong>. Your account has been successfully set up and is ready to use.</p>

<div style="background-color: #f8fafc; border-radius: 12px; padding: 16px; margin: 20px 0; border: 1px solid #e2e8f0; font-size: 14px;">
  <div style="margin-bottom: 8px;"><strong>Email:</strong> <span style="color: #334155;">{email}</span></div>
  <div style="margin-bottom: 8px;"><strong>Collection / Table:</strong> <span style="font-family: monospace; background: #e2e8f0; padding: 2px 6px; border-radius: 4px;">{collection}</span></div>
  <div><strong>Status:</strong> <span style="color: #059669; font-weight: 600;">Active</span></div>
</div>

<p>You can now log in to your account and explore all available features:</p>
<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Go to Dashboard &rarr;</a>
</div>

<p style="color: #64748b; font-size: 13px; margin-top: 30px;">If you did not register for this account, please contact your administrator or ignore this email.</p>"#.to_string(),
    }
}

pub fn default_forgot_password_template() -> EmailTemplate {
    EmailTemplate {
        subject: "Password Reset Request - {appName}".to_string(),
        body: r#"<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Password Reset Request 🔐</h2>
<p>We received a request to reset the password for your account (<strong>{email}</strong>).</p>

<p>Use the following secure token to complete your password reset:</p>

<div class="token-box">
  {token}
</div>

<p style="font-size: 13px; color: #64748b;">Copy and paste this token into the password reset form in your application, or submit it to the <code>/reset-password</code> endpoint.</p>

<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Open Application</a>
</div>

<div style="margin-top: 30px; padding: 14px; background-color: #fff1f2; border-left: 4px solid #f43f5e; border-radius: 8px; font-size: 13px; color: #9f1239;">
  <strong>Security note:</strong> If you did not request a password reset, you can safely ignore this email. Your current password will remain unchanged.
</div>"#.to_string(),
    }
}

pub fn default_password_changed_template() -> EmailTemplate {
    EmailTemplate {
        subject: "Security Alert: Password Changed - {appName}".to_string(),
        body: r#"<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Password Successfully Changed 🛡️</h2>
<p>This is a confirmation that the password for your account (<strong>{email}</strong>) was changed on <strong>{date}</strong>.</p>

<div style="margin: 20px 0; padding: 16px; background-color: #f0fdf4; border-left: 4px solid #10b981; border-radius: 8px; font-size: 14px; color: #065f46;">
  Your account is now secured with the new password. All active sessions have been safely updated.
</div>

<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Sign In to Your Account &rarr;</a>
</div>

<p style="color: #64748b; font-size: 13px; margin-top: 25px;">If you did not make this change, please contact your system administrator immediately to secure your account.</p>"#.to_string(),
    }
}

pub fn default_admin_created_template() -> EmailTemplate {
    EmailTemplate {
        subject: "Administrator Access Granted - {appName}".to_string(),
        body: r#"<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Administrator Account Created 👑</h2>
<p>You have been granted administrator access to <strong>{appName}</strong> with the email address <strong>{email}</strong>.</p>

<p>You can now access the management console and configure databases, collections, and security settings:</p>

<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Open Admin Console &rarr;</a>
</div>

<p style="color: #64748b; font-size: 13px; margin-top: 25px;">Please ensure your administrator credentials are stored securely and never shared.</p>"#.to_string(),
    }
}

pub fn default_verification_template() -> EmailTemplate {
    EmailTemplate {
        subject: "Verify your email address - {appName}".to_string(),
        body: r#"<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Verify Your Email Address ✉️</h2>
<p>Thank you for signing up for <strong>{appName}</strong>. Please confirm your email address (<strong>{email}</strong>) to activate your account features.</p>

<div style="text-align: center; margin: 30px 0;">
  <a href="{verifyUrl}" class="btn">Verify Email Address &rarr;</a>
</div>

<p>Or use the following verification token directly in your application:</p>

<div class="token-box">
  {token}
</div>

<p style="font-size: 13px; color: #64748b; margin-top: 20px;">If the button above does not work, copy and paste this link into your browser:<br/><a href="{verifyUrl}" style="color: #059669; word-break: break-all;">{verifyUrl}</a></p>

<div style="margin-top: 30px; padding: 14px; background-color: #f8fafc; border-left: 4px solid #64748b; border-radius: 8px; font-size: 13px; color: #475569;">
  If you did not create an account on {appName}, you can safely ignore this email.
</div>"#.to_string(),
    }
}

/// Template: Email Verification
pub fn render_verification_email(
    custom_tmpl: Option<&EmailTemplate>,
    email: &str,
    verify_token: &str,
    verify_url: &str,
    app_name: &str,
    app_url: &str,
) -> (String, String) {
    let now = chrono::Utc::now().to_rfc3339();

    let (raw_subj, raw_body) = if let Some(tmpl) = custom_tmpl {
        if !tmpl.subject.trim().is_empty() && !tmpl.body.trim().is_empty() {
            (tmpl.subject.clone(), tmpl.body.clone())
        } else {
            let def = default_verification_template();
            (def.subject, def.body)
        }
    } else {
        let def = default_verification_template();
        (def.subject, def.body)
    };

    let pairs = [
        ("{email}", email),
        ("{token}", verify_token),
        ("{verifyUrl}", verify_url),
        ("{appName}", app_name),
        ("{appUrl}", app_url),
        ("{date}", &now),
    ];

    let subject = replace_placeholders(&raw_subj, &pairs);
    let body = replace_placeholders(&raw_body, &pairs);

    let html = if body.contains("<!DOCTYPE") || body.contains("<html") {
        body
    } else {
        render_html_layout(&subject, &body, app_name, app_url)
    };

    (subject, html)
}

/// Template: Welcome Email on User Registration
pub fn render_welcome_email(
    custom_tmpl: Option<&EmailTemplate>,
    name: &str,
    email: &str,
    collection: &str,
    app_name: &str,
    app_url: &str,
) -> (String, String) {
    let now = chrono::Utc::now().to_rfc3339();
    let display_name = if name.is_empty() { email } else { name };

    let (raw_subj, raw_body) = if let Some(tmpl) = custom_tmpl {
        if !tmpl.subject.trim().is_empty() && !tmpl.body.trim().is_empty() {
            (tmpl.subject.clone(), tmpl.body.clone())
        } else {
            let def = default_welcome_template();
            (def.subject, def.body)
        }
    } else {
        let def = default_welcome_template();
        (def.subject, def.body)
    };

    let pairs = [
        ("{name}", display_name),
        ("{email}", email),
        ("{collection}", collection),
        ("{appName}", app_name),
        ("{appUrl}", app_url),
        ("{date}", &now),
    ];

    let subject = replace_placeholders(&raw_subj, &pairs);
    let body = replace_placeholders(&raw_body, &pairs);

    let html = if body.contains("<!DOCTYPE") || body.contains("<html") {
        body
    } else {
        render_html_layout(&subject, &body, app_name, app_url)
    };

    (subject, html)
}

/// Template: Forgot Password / Password Reset Token
pub fn render_forgot_password_email(
    custom_tmpl: Option<&EmailTemplate>,
    email: &str,
    reset_token: &str,
    app_name: &str,
    app_url: &str,
) -> (String, String) {
    let now = chrono::Utc::now().to_rfc3339();

    let (raw_subj, raw_body) = if let Some(tmpl) = custom_tmpl {
        if !tmpl.subject.trim().is_empty() && !tmpl.body.trim().is_empty() {
            (tmpl.subject.clone(), tmpl.body.clone())
        } else {
            let def = default_forgot_password_template();
            (def.subject, def.body)
        }
    } else {
        let def = default_forgot_password_template();
        (def.subject, def.body)
    };

    let pairs = [
        ("{email}", email),
        ("{token}", reset_token),
        ("{appName}", app_name),
        ("{appUrl}", app_url),
        ("{date}", &now),
    ];

    let subject = replace_placeholders(&raw_subj, &pairs);
    let body = replace_placeholders(&raw_body, &pairs);

    let html = if body.contains("<!DOCTYPE") || body.contains("<html") {
        body
    } else {
        render_html_layout(&subject, &body, app_name, app_url)
    };

    (subject, html)
}

/// Template: Password Changed Notification
pub fn render_password_changed_email(
    custom_tmpl: Option<&EmailTemplate>,
    email: &str,
    app_name: &str,
    app_url: &str,
) -> (String, String) {
    let now = chrono::Utc::now().to_rfc3339();

    let (raw_subj, raw_body) = if let Some(tmpl) = custom_tmpl {
        if !tmpl.subject.trim().is_empty() && !tmpl.body.trim().is_empty() {
            (tmpl.subject.clone(), tmpl.body.clone())
        } else {
            let def = default_password_changed_template();
            (def.subject, def.body)
        }
    } else {
        let def = default_password_changed_template();
        (def.subject, def.body)
    };

    let pairs = [
        ("{email}", email),
        ("{appName}", app_name),
        ("{appUrl}", app_url),
        ("{date}", &now),
    ];

    let subject = replace_placeholders(&raw_subj, &pairs);
    let body = replace_placeholders(&raw_body, &pairs);

    let html = if body.contains("<!DOCTYPE") || body.contains("<html") {
        body
    } else {
        render_html_layout(&subject, &body, app_name, app_url)
    };

    (subject, html)
}

/// Template: Admin Account Created Notification
pub fn render_admin_created_email(
    custom_tmpl: Option<&EmailTemplate>,
    email: &str,
    app_name: &str,
    app_url: &str,
) -> (String, String) {
    let now = chrono::Utc::now().to_rfc3339();

    let (raw_subj, raw_body) = if let Some(tmpl) = custom_tmpl {
        if !tmpl.subject.trim().is_empty() && !tmpl.body.trim().is_empty() {
            (tmpl.subject.clone(), tmpl.body.clone())
        } else {
            let def = default_admin_created_template();
            (def.subject, def.body)
        }
    } else {
        let def = default_admin_created_template();
        (def.subject, def.body)
    };

    let pairs = [
        ("{email}", email),
        ("{appName}", app_name),
        ("{appUrl}", app_url),
        ("{date}", &now),
    ];

    let subject = replace_placeholders(&raw_subj, &pairs);
    let body = replace_placeholders(&raw_body, &pairs);

    let html = if body.contains("<!DOCTYPE") || body.contains("<html") {
        body
    } else {
        render_html_layout(&subject, &body, app_name, app_url)
    };

    (subject, html)
}

/// Template: Test Email
pub fn render_test_email(
    to: &str,
    app_name: &str,
    app_url: &str,
) -> (String, String) {
    let subject = format!("Test Email from {}", app_name);
    let time = chrono::Utc::now().to_rfc3339();

    let body = format!(
        r#"<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">SMTP Test Successful! 🚀</h2>
<p>Congratulations! Your SMTP settings on <strong>{app_name}</strong> are working properly.</p>

<div style="background-color: #f8fafc; border-radius: 12px; padding: 16px; margin: 20px 0; border: 1px solid #e2e8f0; font-size: 14px;">
  <div style="margin-bottom: 8px;"><strong>Recipient:</strong> <span style="color: #334155;">{to}</span></div>
  <div style="margin-bottom: 8px;"><strong>Timestamp:</strong> <span style="color: #334155;">{time}</span></div>
  <div><strong>Delivery Status:</strong> <span style="color: #059669; font-weight: 700;">OK (Connected & Delivered)</span></div>
</div>

<p>Your application is now configured to send emails for registration, password recovery, and security alerts.</p>"#,
        app_name = app_name,
        to = to,
        time = time
    );

    let html = render_html_layout(&subject, &body, app_name, app_url);
    (subject, html)
}

/// Helper function to extract app metadata (app_name, app_url) from settings/env.
pub async fn get_app_meta(pool: &DbPool) -> (String, String) {
    let mut app_name = Env::get_or("APP_NAME", "PadiBase Studio");
    let mut app_url = format!(
        "http://{}:{}",
        Env::get_or("APP_HOST", "127.0.0.1"),
        Env::get_or("APP_PORT", "8085")
    );

    if let Ok(settings) = get_system_settings(pool).await {
        if let Some(meta) = settings.get("meta") {
            if let Some(name) = meta.get("appName").and_then(|v| v.as_str()) {
                if !name.trim().is_empty() {
                    app_name = name.to_string();
                }
            }
            if let Some(url) = meta.get("appUrl").and_then(|v| v.as_str()) {
                if !url.trim().is_empty() {
                    app_url = url.to_string();
                }
            }
        }
    }

    (app_name, app_url)
}
