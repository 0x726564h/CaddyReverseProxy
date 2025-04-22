use headless_chrome::{Browser, LaunchOptions, protocol::network::CookieParam};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Конфигурация для плагина Reverse Proxy
#[derive(Debug)]
pub struct ReverseProxyConfig {
    pub target_host: String,
    pub target_port: u16,
    pub use_tor: bool,
    pub tor_proxy: Option<String>, // Адрес SOCKS5 прокси для Tor
}

/// Основная структура Reverse Proxy
pub struct ReverseProxy {
    config: ReverseProxyConfig,
    browser: Arc<Mutex<Browser>>,
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
        Ok(Self {
            config,
            browser: Arc::new(Mutex::new(browser)),
        })
    }

    /// Обработка запроса через Headless Browser
    pub async fn handle_request(
        &self,
        path: &str,
        headers: Vec<(String, String)>,
        cookies: Vec<CookieParam>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let browser = self.browser.clone();
        let mut browser = browser.lock().await;

        let target_url = format!(
            "http://{}:{}{}",
            self.config.target_host, self.config.target_port, path
        );

        // Создаем новую страницу для навигации
        let tab = browser.new_tab()?;
        tab.navigate_to(&target_url)?;
        
        // Устанавливаем cookies
        for cookie in cookies {
            tab.set_cookie(cookie)?;
        }

        // Устанавливаем заголовки
        for (key, value) in headers {
            tab.set_extra_http_headers(vec![(key, value)])?;
        }

        // Ждем загрузки страницы
        tab.wait_until_navigated()?;

        // Получаем HTML содержимое страницы
        let content = tab.get_content()?;
        Ok(content)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Пример конфигурации
    let config = ReverseProxyConfig {
        target_host: "example.onion".to_string(),
        target_port: 80,
        use_tor: true,
        tor_proxy: Some("127.0.0.1:9050".to_string()), // Укажите ваш Tor SOCKS5 прокси
    };

    let reverse_proxy = ReverseProxy::new(config).await?;

    // Пример запроса
    let response = reverse_proxy
        .handle_request(
            "/path",
            vec![("User-Agent".to_string(), "Caddy-Headless".to_string())],
            vec![],
        )
        .await?;

    println!("Response: {}", response);
    Ok(())
}