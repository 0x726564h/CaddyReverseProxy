#[cfg(test)]
mod tests {
    use super::*;
    use headless_chrome::protocol::network::CookieParam;

    #[tokio::test]
    async fn test_reverse_proxy_initialization() {
        let config = ReverseProxyConfig {
            target_host: "example.onion".to_string(),
            target_port: 80,
            use_tor: true,
            tor_proxy: Some("127.0.0.1:9050".to_string()),
            request_timeout: Some(30),
            disable_images: true,
        };

        let reverse_proxy = ReverseProxy::new(config.clone());
        assert!(reverse_proxy.is_ok(), "Reverse Proxy initialization failed!");
    }

    #[tokio::test]
    async fn test_handle_request_success() {
        let config = ReverseProxyConfig {
            target_host: "example.com".to_string(),
            target_port: 80,
            use_tor: false,
            tor_proxy: None,
            request_timeout: Some(30),
            disable_images: false,
        };

        let reverse_proxy = ReverseProxy::new(config).unwrap();

        let response = reverse_proxy
            .handle_request(
                "/path",
                vec![("User-Agent".to_string(), "Caddy-Headless".to_string())],
                vec![],
                None,
            )
            .await;

        assert!(response.is_ok(), "Request failed: {:?}", response.err());
    }

    #[tokio::test]
    async fn test_request_timeout() {
        let config = ReverseProxyConfig {
            target_host: "example.onion".to_string(),
            target_port: 80,
            use_tor: true,
            tor_proxy: Some("127.0.0.1:9050".to_string()),
            request_timeout: Some(1), // Укороченный таймаут
            disable_images: true,
        };

        let reverse_proxy = ReverseProxy::new(config).unwrap();

        let response = reverse_proxy
            .handle_request(
                "/path",
                vec![("User-Agent".to_string(), "Caddy-Headless".to_string())],
                vec![],
                None,
            )
            .await;

        assert!(response.is_err(), "Request should have timed out!");
    }

    #[tokio::test]
    async fn test_set_cookies() {
        let config = ReverseProxyConfig {
            target_host: "example.com".to_string(),
            target_port: 80,
            use_tor: false,
            tor_proxy: None,
            request_timeout: Some(30),
            disable_images: false,
        };

        let reverse_proxy = ReverseProxy::new(config).unwrap();

        let cookie = CookieParam::new("test_cookie".to_string(), "test_value".to_string());
        let response = reverse_proxy
            .handle_request(
                "/path",
                vec![("User-Agent".to_string(), "Caddy-Headless".to_string())],
                vec![cookie],
                None,
            )
            .await;

        assert!(response.is_ok(), "Request with cookies failed: {:?}", response.err());
    }
}