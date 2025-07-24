use reqwest::header;
use tendermint_rpc::HttpClient;

#[derive(Clone, Debug)]
pub struct Client {
    inner: HttpClient,
}

impl Client {
    pub fn new(ur: &str) -> Self {
        let headers = Self::default_headers();

        let url = ur.parse().expect("Invalid URL");
        let inner = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");
        let http_client = HttpClient::new_from_parts(
            inner,
            url,
            tendermint_rpc::client::CompatMode::V0_37,
        );
        Client { inner: http_client }
    }

    pub fn get(&self) -> HttpClient {
        self.inner.clone()
    }

    fn default_headers() -> header::HeaderMap {
        let version = env!("CARGO_PKG_VERSION");

        let mut headers = header::HeaderMap::new();
        headers.insert("x-namada", header::HeaderValue::from_static(version));
        headers.insert(
            "User-Agent",
            header::HeaderValue::from_static("namada-indexer"),
        );
        headers
    }
}

impl AsRef<HttpClient> for Client {
    fn as_ref(&self) -> &HttpClient {
        &self.inner
    }
}
