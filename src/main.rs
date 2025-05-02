use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    routing::get,
};
use dotenv::dotenv;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::interval;
use tower_http::trace::TraceLayer;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    prometheus: PrometheusConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct PrometheusConfig {
    url: String,
    queries: HashMap<String, QueryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryConfig {
    query: String,
    interval: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct PrometheusData {
    status: String,
    data: serde_json::Value,
    timestamp: u64,
}

struct AppState {
    prometheus_data: Mutex<HashMap<String, PrometheusData>>,
}

async fn get_metric(
    State(state): State<Arc<AppState>>,
    AxumPath(metric_name): AxumPath<String>,
) -> Result<Json<PrometheusData>, axum::http::StatusCode> {
    let prometheus_data = state.prometheus_data.lock().unwrap();
    prometheus_data
        .get(&metric_name)
        .map(|data| Json(data.clone()))
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn query_prometheus(
    prometheus_url: &str,
    prometheus_query: &str,
) -> Result<PrometheusData, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/query", prometheus_url);
    let response = client
        .get(&url)
        .query(&[("query", prometheus_query)])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    Ok(PrometheusData {
        status: "success".to_string(),
        data: response,
        timestamp: now,
    })
}

async fn prometheus_poller(
    app_state: Arc<AppState>,
    prometheus_url: String,
    query_name: String,
    query_config: QueryConfig,
) {
    let mut interval = interval(Duration::from_millis(query_config.interval));
    let query = query_config.query;

    loop {
        interval.tick().await;
        info!("Querying Prometheus for {}: {}", query_name, query);
        match query_prometheus(&prometheus_url, &query).await {
            Ok(data) => {
                if let Ok(mut prometheus_data) = app_state.prometheus_data.lock() {
                    prometheus_data.insert(query_name.clone(), data);
                    info!("Updated Prometheus data for {}", query_name);
                }
            }
            Err(e) => {
                error!("Failed to query Prometheus for {}: {}", query_name, e);
            }
        }
    }
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&config_content)?;
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();

    let config_path =
        env::var("CONFIG_PATH").expect("CONFIG_PATH environment variable must be set");
    let config = load_config(config_path)?;

    info!(
        "Starting Prometheus proxy with URL: {}, {} queries configured",
        config.prometheus.url,
        config.prometheus.queries.len()
    );

    let app_state = Arc::new(AppState {
        prometheus_data: Mutex::new(HashMap::new()),
    });

    for (query_name, query_config) in &config.prometheus.queries {
        tokio::spawn({
            let state = app_state.clone();
            let url = config.prometheus.url.clone();
            let name = query_name.clone();
            let config = query_config.clone();

            async move {
                prometheus_poller(state, url, name, config).await;
            }
        });
    }

    let app = Router::new()
        .route("/health", get(|| async { "Healthy" }))
        .route("/metrics/{metric_name}", get(get_metric))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
