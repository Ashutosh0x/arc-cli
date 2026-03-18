use reqwest::Client;
use std::sync::OnceLock;
use std::time::Duration;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

pub fn client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            // HTTP/2 multiplexing — single TCP connection, many streams
            .http2_prior_knowledge()
            // Keep-alive for connection reuse
            .pool_max_idle_per_host(4)
            .pool_idle_timeout(Duration::from_secs(90))
            // TCP_NODELAY — disable Nagle's for lower latency
            .tcp_nodelay(true)
            // Aggressive timeouts
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(120))
            .brotli(true)
            .gzip(true)
            .build()
            .expect("Failed to build global HTTP client pool")
    })
}
