# Caddy Reverse Proxy Plugin на Rust

## Описание

Этот плагин реализует обратное проксирование через Headless Browser с использованием Tor (через прокси LibTor).

### Возможности:
- Поддержка onion-ресурсов через Tor.
- Отключение GPU, аудио и изображений для оптимизации.
- Пересылка cookies и безопасных заголовков.
- Настраиваемый таймаут для запросов.

---

## Установка и Сборка

### Установка Rust
Убедитесь, что у вас установлен Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Установка Tor
Установите Tor и запустите его:
```bash
sudo apt install tor
tor
```
Проверьте, что Tor слушает на `127.0.0.1:9050`.

### Клонирование репозитория и сборка
1. Клонируйте репозиторий:
   ```bash
   git clone https://github.com/0x726564h/CaddyReverseProxy
   cd CaddyReverseProxy
   ```

2. Соберите проект:
   ```bash
   cargo build --release
   ```

---

## Использование

### Запуск плагина
Для запуска плагина выполните:
```bash
cargo run --release
```

### Конфигурация
Пример конфигурации можно найти в файле `main.rs`. Ключевые параметры:
- `target_host`: целевой сервер (например, `example.onion`).
- `target_port`: порт целевого сервера.
- `use_tor`: использовать Tor для доступа к ресурсам.
- `tor_proxy`: адрес SOCKS5-прокси для Tor (например, `127.0.0.1:9050`).
- `request_timeout`: таймаут запроса в секундах.
- `disable_images`: отключить загрузку изображений.

### Пример Запроса
Пример запроса через Reverse Proxy:
```rust
let response = reverse_proxy
    .handle_request(
        "/path",
        vec![("User-Agent".to_string(), "Caddy-Headless".to_string())],
        vec![],
        None,
    )
    .await?;
println!("Response: {}", response);
```

---

## Тестирование

### Запуск тестов
Для запуска тестов выполните:
```bash
cargo test
```

Тесты включают:
- Проверку корректности обработки запросов.
- Проверку работы с cookies.
- Проверку таймаутов и ошибок.

---

## Лицензия

Этот проект распространяется под лицензией MIT.
