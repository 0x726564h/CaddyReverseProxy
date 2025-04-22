use std::env;
use std::sync::Arc;

use actix_web::{App, HttpServer};
use env_logger;
use log::info;

mod config;
mod server;
mod proxy;
mod browser;
mod tor;
mod utils;

use config::Config;
use server::configure_server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    info!("Starting Caddy Reverse Proxy Plugin...");

    let config = if let Some(path) = env::args().nth(1) {
        Arc::new(Config::from_file(&path).expect("Failed to load configuration from file"))
    } else {
        Arc::new(Config::from_env().expect("Failed to load configuration from environment"))
    };

    info!("Configuration loaded: {:?}", config);

    HttpServer::new(move || {
        App::new()
            .app_data(config.clone())
            .configure(configure_server)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// config.rs
mod config {
    use serde::{Deserialize, Serialize};
    use std::env;
    use std::fs;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Config {
        pub target_host: String,
        pub target_port: u16,
        pub use_tor: bool,
        pub tor_proxy: String,
        pub request_timeout: u64,
        pub disable_images: bool,
    }

    impl Config {
        pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let content = fs::read_to_string(path)?;
            let config = serde_json::from_str(&content)?;
            Ok(config)
        }

        pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
            let target_host = env::var("TARGET_HOST")?;
            let target_port = env::var("TARGET_PORT")?.parse()?;
            let use_tor = env::var("USE_TOR").map(|v| v == "true").unwrap_or(false);
            let tor_proxy = env::var("TOR_PROXY").unwrap_or("127.0.0.1:9050".to_string());
            let request_timeout = env::var("REQUEST_TIMEOUT").map(|v| v.parse().unwrap_or(30)).unwrap_or(30);
            let disable_images = env::var("DISABLE_IMAGES").map(|v| v == "true").unwrap_or(true);

            Ok(Config {
                target_host,
                target_port,
                use_tor,
                tor_proxy,
                request_timeout,
                disable_images,
            })
        }
    }
}

// server.rs
mod server {
    use actix_web::{web, HttpRequest, HttpResponse};
    use log::info;

    use crate::proxy::proxy_request;
    use crate::Config;

    pub fn configure_server(cfg: &mut web::ServiceConfig) {
        cfg.service(web::resource("/{path:.*}").route(web::get().to(handle_request)));
    }

    async fn handle_request(req: HttpRequest, config: web::Data<Arc<Config>>) -> HttpResponse {
        let path = req.match_info().get("path").unwrap_or("");
        info!("Received request for path: {}", path);

        match proxy_request(&req, &config).await {
            Ok(response) => HttpResponse::Ok()
                .content_type("text/html")
                .body(response),
            Err(e) => {
                log::error!("Proxy error: {}", e);
                HttpResponse::InternalServerError().body(format!("Error: {}", e))
            }
        }
    }
}

// proxy.rs
mod proxy {
    use actix_web::HttpRequest;

    use crate::browser::Browser;
    use crate::Config;

    pub async fn proxy_request(req: &HttpRequest, config: &Config) -> Result<String, Box<dyn std::error::Error>> {
        let mut browser = Browser::new(config)?;
        let url = format!("http://{}:{}/{}", config.target_host, config.target_port, req.uri().path());
        
        let response = browser.navigate(&url, config.request_timeout).await?;
        Ok(response)
    }
}

// browser.rs
mod browser {
    use headless_chrome::{Browser as ChromeBrowser, LaunchOptions};
    use log::info;

    use crate::tor::setup_tor_proxy;
    use crate::Config;

    pub struct Browser {
        browser: ChromeBrowser,
    }

    impl Browser {
        pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
            let mut options = LaunchOptions::default_builder();

            if config.use_tor {
                options = options.proxy_server(setup_tor_proxy(&config.tor_proxy)?);
            }

            if config.disable_images {
                options = options.args(vec!["--disable-images"]);
            }

            let browser = ChromeBrowser::new(options.build()?)?;
            info!("Headless browser initialized with config: {:?}", config);
            Ok(Browser { browser })
        }

        pub async fn navigate(&mut self, url: &str, timeout: u64) -> Result<String, Box<dyn std::error::Error>> {
            let tab = self.browser.new_tab()?;
            tab.set_default_timeout(std::time::Duration::from_secs(timeout));
            let content = tab.navigate_to(url)?.wait_until_navigated()?.get_content()?;
            Ok(content)
        }
    }
}

// tor.rs
mod tor {
    use std::net::SocketAddr;

    pub fn setup_tor_proxy(proxy_addr: &str) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        let addr: SocketAddr = proxy_addr.parse()?;
        Ok(addr)
    }
}

// utils.rs
mod utils {
    use log::error;

    pub fn log_error(e: &dyn std::error::Error) {
        error!("Error occurred: {}", e);
    }
}
