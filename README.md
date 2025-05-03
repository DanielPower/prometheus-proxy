# Prometheus Proxy

A simple proxy service that queries Prometheus metrics and caches the results, exposing them via a REST API. Built with Axum web framework.

## Features

- Queries Prometheus at a configurable interval (default: 5000 milliseconds)
- Caches the query results
- Exposes the cached results via a REST API
- Configurable via environment variables

## Configuration

The application uses a YAML configuration file for settings. The path to this file is specified using an environment variable:

- `CONFIG_PATH`: Path to the YAML configuration file

The configuration file should have the following structure:

```yaml
prometheus:
  url: http://localhost:9090  # URL of the Prometheus server
  queries:                    # Map of named queries to execute
    health:                   # Name of the query (used in API endpoint)
      query: up               # Prometheus query to execute
      interval: 1000          # Polling interval in milliseconds
    kube_node_info:
      query: kube_node_info
      interval: 5000
    ready_nodes:
      query: kube_node_status_condition{condition="Ready",status="true"}
      interval: 5000
```

Each query will be polled at its specified interval (in milliseconds).

The environment variable can be set in a `.env` file in the project root.

## API Endpoints

- `/metrics/:metric_name`: Returns the cached result for a specific named query (e.g., `/metrics/health`)
- `/health`: Health check endpoint

## Building and Running

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run
```

Or with direct environment variables:

```bash
PROMETHEUS_URL=http://localhost:9090 PROMETHEUS_QUERY=up cargo run
```

## Docker

To build and run with Docker:

```bash
# Build the Docker image
docker build -t prometheus-proxy .

# Run the container with your config file mounted
docker run -p 8080:8080 -v $(pwd)/config.yaml:/etc/prometheus-proxy/config.yaml prometheus-proxy
```