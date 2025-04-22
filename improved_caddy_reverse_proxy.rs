use headless_chrome::{Browser, LaunchOptions, protocol::network::CookieParam};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, error};

/// Конфигурация для плагина Reverse Proxy
#[derive(Debug)]
pub struct ReverseProxyConfig {
    pub target_host: String,
    pub target_port: u16,
    pub use_tor: bool,
    pub tor_proxy: Option<String>, // Адрес SOCKS5 прокси для Tor
    pub request_timeout: Option<u64>, // Таймаут запросов в секундах
}

/// Глобальный браузер для повторного использования
static BROWSER: OnceCell<Arc<Mutex<Browser>>> = OnceCell::new();

/// Основная структура Reverse Proxy
pub struct ReverseProxy {
    config: ReverseProxyConfig,
}

impl ReverseProxy {
    /// Создание нового Reverse Proxy
    pub async fn new(config: ReverseProxyConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Настраиваем параметры запуска браузера
        let mut launch_options = LaunchOptions::default_builder()
            .headless(true) // Включаем headless режим
            .disable_gpu(true) // Отключаем использование GPU
            .disable_audio(true) // Отключаем аудио
            .sandbox(false) // Отключаем песочницу (внимание, небезопасно)
            .build()
            .unwrap();

        if config.use_tor {
            if let Some(tor_proxy) = &config.tor_proxy {
                // Устанавливаем прокси для Tor
                launch_options.args.push(format!("--proxy-server=socks5://{}", tor_proxy));
            } else {
                panic!("Tor proxy must be specified when use_tor is enabled.");
            }
        }

        let browser = Browser::new(launch_options)?;
        BROWSER.set(Arc::new(Mutex::new(browser))).unwrap();

        info!("Reverse Proxy initialized with configuration: {:?}", config);

        Ok(Self { config })
    }

    /// Обработка запроса через Headless Browser
    pub async fn handle_request(
        &self,
        path: &str,
        headers: Vec<(String, String)>,
        cookies: Vec<CookieParam>,
        body: Option<String>, // Поддержка тела запроса
    ) -> Result<String, Box<dyn std::error::Error>> {
        let browser = BROWSER.get().unwrap().clone();
        let mut browser = browser.lock().await;

        let target_url = format!(
            "http://{}:{}{}",
            self.config.target_host, self.config.target_port, path
        );

        // Создаем новую страницу для навигации
        let tab = match browser.new_tab() {
            Ok(tab) => tab,
            Err(err) => {
                error!("Failed to create new tab: {}", err);
                return Err(Box::new(err));
            }
        };

        // Устанавливаем cookies
        for cookie in &cookies {
            tab.set_cookie(cookie.clone())?;
        }

        // Устанавливаем заголовки
        tab.set_extra_http_headers(headers.clone())?;

        // Выполняем запрос
        if let Some(body_content) = body {
            tab.navigate_to_with_post_data(&target_url, body_content.as_bytes())?;
        } else {
            tab.navigate_to(&target_url)?;
        }

        // Ждем загрузки страницы с учетом таймаута
        if let Some(timeout) = self.config.request_timeout {
            let result = tab.wait_until_navigated_with_timeout(std::time::Duration::from_secs(timeout));
            if result.is_err() {
                error!("Request timed out after {} seconds", timeout);
                return Err(Box::new(result.err().unwrap()));
            }
        } else {
            tab.wait_until_navigated()?;
        }

        // Получаем HTML содержимое страницы
        let content = tab.get_content()?;
        Ok(content)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация логирования
    env_logger::init();

    // Пример конфигурации
    let config = ReverseProxyConfig {
        target_host: "example.onion".to_string(),
        target_port: 80,
        use_tor: true,
        tor_proxy: Some("127.0.0.1:9050".to_string()), // Укажите ваш Tor SOCKS5 прокси
        request_timeout: Some(30), // Таймаут в 30 секунд
    };

    let reverse_proxy = ReverseProxy::new(config).await?;

    // Пример запроса
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