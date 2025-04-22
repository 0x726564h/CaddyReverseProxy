use headless_chrome::{Browser, LaunchOptions, protocol::network::CookieParam};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::fs;
use std::sync::{Arc, Mutex};
use tokio::task;
use log::{info, error};

/// Структура конфигурации для TOML
#[derive(Debug, Deserialize)]
struct ConfigFile {
    target_host: String,
    target_port: u16,
    use_tor: bool,
    tor_proxy: Option<String>, // Адрес SOCKS5 прокси для Tor
    request_timeout: Option<u64>, // Таймаут запросов в секундах
    disable_images: bool, // Отключение загрузки изображений
}

/// Конфигурация для плагина Reverse Proxy
#[derive(Debug, Clone)]
pub struct ReverseProxyConfig {
    pub target_host: String,
    pub target_port: u16,
    pub use_tor: bool,
    pub tor_proxy: Option<String>, // Адрес SOCKS5 прокси для Tor
    pub request_timeout: Option<u64>, // Таймаут запросов в секундах
    pub disable_images: bool, // Отключение загрузки изображений
}

/// Глобальный браузер для повторного использования
static BROWSER: OnceCell<Arc<Mutex<Browser>>> = OnceCell::new();

/// Основная структура Reverse Proxy
pub struct ReverseProxy {
    config: ReverseProxyConfig,
}

impl ReverseProxy {
    /// Создание нового Reverse Proxy
    pub fn new(config: ReverseProxyConfig) -> Result<Self, Box<dyn std::error::Error>> {
        if config.use_tor && config.tor_proxy.is_none() {
            error!("Tor proxy must be specified when use_tor is enabled.");
            return Err("Invalid configuration: Tor proxy is required.".into());
        }

        let mut launch_options = LaunchOptions::default_builder()
            .headless(true)
            .disable_gpu(true)
            .disable_audio(true)
            .sandbox(false)
            .build()
            .unwrap();

        if config.use_tor {
            if let Some(tor_proxy) = &config.tor_proxy {
                launch_options.args.push(format!("--proxy-server=socks5://{}", tor_proxy));
            }
        }

        BROWSER.get_or_try_init(|| {
            let browser = Browser::new(launch_options)?;
            Ok(Arc::new(Mutex::new(browser)))
        })?;

        info!("Reverse Proxy initialized with configuration: {:?}", config);

        Ok(Self { config })
    }

    /// Обработка запроса через Headless Browser
    pub async fn handle_request(
        &self,
        path: &str,
        headers: Vec<(String, String)>,
        cookies: Vec<CookieParam>,
        body: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let browser = BROWSER.get().unwrap().clone();
        let mut browser = browser.lock().unwrap();

        let target_url = format!(
            "http://{}:{}{}",
            self.config.target_host, self.config.target_port, path
        );

        let tab = match browser.new_tab() {
            Ok(tab) => tab,
            Err(err) => {
                error!("Failed to create new tab: {}", err);
                return Err(Box::new(err));
            }
        };

        if self.config.disable_images {
            tab.enable_request_interception()?;
            tab.intercept_requests(|request| {
                if request.resource_type == "Image" {
                    Ok(request.abort())
                } else {
                    Ok(request.continue_request())
                }
            })?;
        }

        for cookie in &cookies {
            tab.set_cookie(cookie.clone())?;
        }

        tab.set_extra_http_headers(headers.clone())?;

        if let Some(body_content) = body {
            tab.navigate_to_with_post_data(&target_url, body_content.as_bytes())?;
        } else {
            tab.navigate_to(&target_url)?;
        }

        if let Some(timeout) = self.config.request_timeout {
            let result = tab.wait_until_navigated_with_timeout(std::time::Duration::from_secs(timeout));
            if result.is_err() {
                error!("Request timed out after {} seconds", timeout);
                return Err(Box::new(result.err().unwrap()));
            }
        } else {
            tab.wait_until_navigated()?;
        }

        let content = tab.get_content()?;
        Ok(content)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config_content = fs::read_to_string("config.toml")?;
    let config_file: ConfigFile = toml::from_str(&config_content)?;

    let config = ReverseProxyConfig {
        target_host: config_file.target_host,
        target_port: config_file.target_port,
        use_tor: config_file.use_tor,
        tor_proxy: config_file.tor_proxy,
        request_timeout: config_file.request_timeout,
        disable_images: config_file.disable_images,
    };

    let reverse_proxy = ReverseProxy::new(config)?;

    let response = reverse_proxy
        .handle_request(
            "/path",
            vec![("User-Agent".to_string(), "Caddy-Headless".to_string())],
            vec![],
            None,
        )
        .await?;

    info!("Response: {}", response);
    Ok(())
}
