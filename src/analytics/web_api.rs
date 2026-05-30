// src/analytics/web_api.rs
//! Simple HTTP API for external access to simulation data.
//!
//! Provides a lightweight REST-like API using only standard library.
//! For production use, consider integrating with axum, actix-web, or warp.
//!
//! # Endpoints
//! - GET /status - Simulation status
//! - GET /population - Population data
//! - GET /agents - List of agents
//! - GET /agents/:id - Single agent details
//! - GET /metrics - Current metrics
//! - GET /events - Recent events
//! - POST /control/pause - Pause simulation
//! - POST /control/resume - Resume simulation
//! - POST /control/step - Single step

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// API server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Address to bind to
    pub bind_address: String,
    /// Port to listen on
    pub port: u16,
    /// Enable CORS headers
    pub enable_cors: bool,
    /// Maximum request body size
    pub max_body_size: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            enable_cors: true,
            max_body_size: 1024 * 1024, // 1 MB
        }
    }
}

/// HTTP response
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    pub fn ok(body: String) -> Self {
        Self {
            status: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body,
        }
    }

    pub fn json<T: Serialize>(data: &T) -> Self {
        let body = serde_json::to_string_pretty(data).unwrap_or_default();
        let mut resp = Self::ok(body);
        resp.headers.insert("Content-Type".to_string(), "application/json".to_string());
        resp
    }

    pub fn not_found(message: &str) -> Self {
        Self {
            status: 404,
            status_text: "Not Found".to_string(),
            headers: HashMap::new(),
            body: message.to_string(),
        }
    }

    pub fn bad_request(message: &str) -> Self {
        Self {
            status: 400,
            status_text: "Bad Request".to_string(),
            headers: HashMap::new(),
            body: message.to_string(),
        }
    }

    pub fn internal_error(message: &str) -> Self {
        Self {
            status: 500,
            status_text: "Internal Server Error".to_string(),
            headers: HashMap::new(),
            body: message.to_string(),
        }
    }

    pub fn with_cors(mut self) -> Self {
        self.headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        self.headers.insert("Access-Control-Allow-Methods".to_string(), "GET, POST, OPTIONS".to_string());
        self.headers.insert("Access-Control-Allow-Headers".to_string(), "Content-Type".to_string());
        self
    }

    pub fn to_http(&self) -> String {
        let mut response = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status, self.status_text
        );

        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }

        response.push_str(&format!("Content-Length: {}\r\n", self.body.len()));
        response.push_str("\r\n");
        response.push_str(&self.body);

        response
    }
}

/// HTTP request
#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpRequest {
    /// Parse path parameters (e.g., /agents/:id)
    pub fn path_param(&self, pattern: &str) -> Option<String> {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = self.path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return None;
        }

        for (i, part) in pattern_parts.iter().enumerate() {
            if part.starts_with(':') {
                return Some(path_parts[i].to_string());
            } else if *part != path_parts[i] {
                return None;
            }
        }

        None
    }

    /// Check if path matches pattern
    pub fn matches(&self, pattern: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = self.path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return false;
        }

        for (i, part) in pattern_parts.iter().enumerate() {
            if !part.starts_with(':') && *part != path_parts[i] {
                return false;
            }
        }

        true
    }
}

/// Parse an HTTP request from a stream
fn parse_request(stream: &mut TcpStream) -> Option<HttpRequest> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;

    let parts: Vec<&str> = first_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let method = parts[0].to_string();
    let full_path = parts[1];

    // Parse path and query string
    let (path, query) = if let Some(idx) = full_path.find('?') {
        let path = full_path[..idx].to_string();
        let query_str = &full_path[idx + 1..];
        let query = parse_query_string(query_str);
        (path, query)
    } else {
        (full_path.to_string(), HashMap::new())
    };

    // Parse headers
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(idx) = line.find(':') {
            let key = line[..idx].trim().to_lowercase();
            let value = line[idx + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    // Parse body if present
    let body = if let Some(len_str) = headers.get("content-length") {
        if let Ok(len) = len_str.parse::<usize>() {
            let mut body = vec![0u8; len];
            reader.read_exact(&mut body).ok()?;
            String::from_utf8(body).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    Some(HttpRequest {
        method,
        path,
        query,
        headers,
        body,
    })
}

fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for part in query.split('&') {
        if let Some(idx) = part.find('=') {
            let key = part[..idx].to_string();
            let value = part[idx + 1..].to_string();
            params.insert(key, value);
        }
    }
    params
}

/// Simulation state provider trait
pub trait SimulationDataProvider: Send + Sync {
    /// Get simulation status
    fn get_status(&self) -> SimulationStatus;

    /// Get population summary
    fn get_population(&self) -> PopulationSummary;

    /// Get list of agents
    fn get_agents(&self) -> Vec<AgentSummary>;

    /// Get single agent by ID
    fn get_agent(&self, id: Uuid) -> Option<AgentDetail>;

    /// Get current metrics
    fn get_metrics(&self) -> MetricsSummary;

    /// Get recent events
    fn get_events(&self, limit: usize) -> Vec<EventSummary>;

    /// Pause simulation
    fn pause(&self) -> bool;

    /// Resume simulation
    fn resume(&self) -> bool;

    /// Single step
    fn step(&self) -> bool;
}

/// Simulation status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationStatus {
    pub running: bool,
    pub paused: bool,
    pub current_tick: u64,
    pub ticks_per_second: f32,
    pub uptime_seconds: u64,
}

/// Population summary response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationSummary {
    pub total: usize,
    pub alive: usize,
    pub births: u64,
    pub deaths: u64,
    pub average_age: f32,
    pub average_health: f32,
    pub average_happiness: f32,
}

/// Agent summary for list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSummary {
    pub id: String,
    pub position: (i32, i32, i32),
    pub health: f32,
    pub age: u32,
    pub is_alive: bool,
}

/// Detailed agent info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDetail {
    pub id: String,
    pub position: (i32, i32, i32),
    pub health: f32,
    pub energy: f32,
    pub age: u32,
    pub is_alive: bool,
    pub drives: HashMap<String, f32>,
    pub inventory_items: u32,
    pub relationships: usize,
    pub traits: Vec<String>,
}

/// Metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub tick: u64,
    pub population: usize,
    pub average_happiness: f32,
    pub average_health: f32,
    pub emergent_patterns: Vec<String>,
}

/// Event summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub id: String,
    pub tick: u64,
    pub event_type: String,
    pub description: String,
    pub severity: f32,
}

/// API Server
pub struct ApiServer {
    config: ApiConfig,
    running: Arc<AtomicBool>,
    provider: Arc<dyn SimulationDataProvider>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(config: ApiConfig, provider: Arc<dyn SimulationDataProvider>) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            provider,
        }
    }

    /// Start the server (blocking)
    pub fn start(&self) -> std::io::Result<()> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let listener = TcpListener::bind(&addr)?;

        self.running.store(true, Ordering::SeqCst);
        println!("API server listening on http://{}", addr);

        for stream in listener.incoming() {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            match stream {
                Ok(mut stream) => {
                    let provider = self.provider.clone();
                    let enable_cors = self.config.enable_cors;

                    thread::spawn(move || {
                        if let Err(e) = handle_request(&mut stream, provider, enable_cors) {
                            eprintln!("Error handling request: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Connection failed: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Start the server in a background thread
    pub fn start_background(self) -> std::thread::JoinHandle<()> {
        thread::spawn(move || {
            if let Err(e) = self.start() {
                eprintln!("API server error: {}", e);
            }
        })
    }

    /// Stop the server
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

fn handle_request(
    stream: &mut TcpStream,
    provider: Arc<dyn SimulationDataProvider>,
    enable_cors: bool,
) -> std::io::Result<()> {
    let request = match parse_request(stream) {
        Some(r) => r,
        None => {
            let response = HttpResponse::bad_request("Invalid request");
            stream.write_all(response.to_http().as_bytes())?;
            return Ok(());
        }
    };

    let mut response = route_request(&request, &provider);

    if enable_cors {
        response = response.with_cors();
    }

    stream.write_all(response.to_http().as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn route_request(
    request: &HttpRequest,
    provider: &Arc<dyn SimulationDataProvider>,
) -> HttpResponse {
    // Handle OPTIONS for CORS preflight
    if request.method == "OPTIONS" {
        return HttpResponse::ok(String::new()).with_cors();
    }

    match (request.method.as_str(), request.path.as_str()) {
        // Status endpoint
        ("GET", "/status") | ("GET", "/") => {
            let status = provider.get_status();
            HttpResponse::json(&status)
        }

        // Population endpoint
        ("GET", "/population") => {
            let population = provider.get_population();
            HttpResponse::json(&population)
        }

        // Agents list
        ("GET", "/agents") => {
            let agents = provider.get_agents();
            HttpResponse::json(&agents)
        }

        // Single agent - check pattern
        ("GET", _path) if request.matches("/agents/:id") => {
            if let Some(id_str) = request.path_param("/agents/:id") {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    if let Some(agent) = provider.get_agent(id) {
                        HttpResponse::json(&agent)
                    } else {
                        HttpResponse::not_found("Agent not found")
                    }
                } else {
                    HttpResponse::bad_request("Invalid agent ID")
                }
            } else {
                HttpResponse::not_found("Not found")
            }
        }

        // Metrics endpoint
        ("GET", "/metrics") => {
            let metrics = provider.get_metrics();
            HttpResponse::json(&metrics)
        }

        // Events endpoint
        ("GET", "/events") => {
            let limit = request
                .query
                .get("limit")
                .and_then(|s| s.parse().ok())
                .unwrap_or(100);
            let events = provider.get_events(limit);
            HttpResponse::json(&events)
        }

        // Control endpoints
        ("POST", "/control/pause") => {
            let success = provider.pause();
            HttpResponse::json(&serde_json::json!({ "success": success }))
        }

        ("POST", "/control/resume") => {
            let success = provider.resume();
            HttpResponse::json(&serde_json::json!({ "success": success }))
        }

        ("POST", "/control/step") => {
            let success = provider.step();
            HttpResponse::json(&serde_json::json!({ "success": success }))
        }

        // API documentation
        ("GET", "/api") => {
            let docs = ApiDocumentation::default();
            HttpResponse::json(&docs)
        }

        // Not found
        _ => HttpResponse::not_found(&format!("Endpoint not found: {} {}", request.method, request.path)),
    }
}

/// API documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocumentation {
    pub version: String,
    pub endpoints: Vec<EndpointDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDoc {
    pub method: String,
    pub path: String,
    pub description: String,
}

impl Default for ApiDocumentation {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            endpoints: vec![
                EndpointDoc {
                    method: "GET".to_string(),
                    path: "/status".to_string(),
                    description: "Get simulation status".to_string(),
                },
                EndpointDoc {
                    method: "GET".to_string(),
                    path: "/population".to_string(),
                    description: "Get population summary".to_string(),
                },
                EndpointDoc {
                    method: "GET".to_string(),
                    path: "/agents".to_string(),
                    description: "List all agents".to_string(),
                },
                EndpointDoc {
                    method: "GET".to_string(),
                    path: "/agents/:id".to_string(),
                    description: "Get agent details by ID".to_string(),
                },
                EndpointDoc {
                    method: "GET".to_string(),
                    path: "/metrics".to_string(),
                    description: "Get current metrics".to_string(),
                },
                EndpointDoc {
                    method: "GET".to_string(),
                    path: "/events?limit=N".to_string(),
                    description: "Get recent events".to_string(),
                },
                EndpointDoc {
                    method: "POST".to_string(),
                    path: "/control/pause".to_string(),
                    description: "Pause simulation".to_string(),
                },
                EndpointDoc {
                    method: "POST".to_string(),
                    path: "/control/resume".to_string(),
                    description: "Resume simulation".to_string(),
                },
                EndpointDoc {
                    method: "POST".to_string(),
                    path: "/control/step".to_string(),
                    description: "Single step simulation".to_string(),
                },
            ],
        }
    }
}

/// Mock provider for testing
#[derive(Default)]
pub struct MockDataProvider {
    pub paused: AtomicBool,
}

impl SimulationDataProvider for MockDataProvider {
    fn get_status(&self) -> SimulationStatus {
        SimulationStatus {
            running: true,
            paused: self.paused.load(Ordering::SeqCst),
            current_tick: 1000,
            ticks_per_second: 60.0,
            uptime_seconds: 3600,
        }
    }

    fn get_population(&self) -> PopulationSummary {
        PopulationSummary {
            total: 50,
            alive: 48,
            births: 10,
            deaths: 2,
            average_age: 500.0,
            average_health: 75.0,
            average_happiness: 0.6,
        }
    }

    fn get_agents(&self) -> Vec<AgentSummary> {
        vec![
            AgentSummary {
                id: Uuid::new_v4().to_string(),
                position: (10, 20, 0),
                health: 100.0,
                age: 500,
                is_alive: true,
            }
        ]
    }

    fn get_agent(&self, _id: Uuid) -> Option<AgentDetail> {
        Some(AgentDetail {
            id: _id.to_string(),
            position: (10, 20, 0),
            health: 100.0,
            energy: 80.0,
            age: 500,
            is_alive: true,
            drives: [("Hunger".to_string(), 0.3)].into_iter().collect(),
            inventory_items: 5,
            relationships: 3,
            traits: vec!["Curious".to_string()],
        })
    }

    fn get_metrics(&self) -> MetricsSummary {
        MetricsSummary {
            tick: 1000,
            population: 50,
            average_happiness: 0.6,
            average_health: 75.0,
            emergent_patterns: vec!["StableEquilibrium".to_string()],
        }
    }

    fn get_events(&self, _limit: usize) -> Vec<EventSummary> {
        vec![
            EventSummary {
                id: Uuid::new_v4().to_string(),
                tick: 999,
                event_type: "AgentBorn".to_string(),
                description: "New agent born".to_string(),
                severity: 0.5,
            }
        ]
    }

    fn pause(&self) -> bool {
        self.paused.store(true, Ordering::SeqCst);
        true
    }

    fn resume(&self) -> bool {
        self.paused.store(false, Ordering::SeqCst);
        true
    }

    fn step(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_response() {
        let response = HttpResponse::json(&serde_json::json!({"test": "value"}));
        assert_eq!(response.status, 200);
        assert!(response.body.contains("test"));
    }

    #[test]
    fn test_path_matching() {
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/agents/123e4567-e89b-12d3-a456-426614174000".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };

        assert!(request.matches("/agents/:id"));
        assert!(!request.matches("/agents"));
        assert!(!request.matches("/agents/:id/details"));

        let id = request.path_param("/agents/:id");
        assert_eq!(id, Some("123e4567-e89b-12d3-a456-426614174000".to_string()));
    }

    #[test]
    fn test_query_string() {
        let query = parse_query_string("limit=100&offset=50");
        assert_eq!(query.get("limit"), Some(&"100".to_string()));
        assert_eq!(query.get("offset"), Some(&"50".to_string()));
    }

    #[test]
    fn test_mock_provider() {
        let provider = MockDataProvider::default();

        let status = provider.get_status();
        assert!(status.running);
        assert!(!status.paused);

        provider.pause();
        let status = provider.get_status();
        assert!(status.paused);

        provider.resume();
        let status = provider.get_status();
        assert!(!status.paused);
    }
}
