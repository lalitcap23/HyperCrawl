use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::collections::HashMap;

use crate::model::LinkGraph;

/// Crawl job request
#[derive(Debug, Deserialize)]
pub struct CrawlRequest {
    pub url: String,
    #[serde(default = "default_max_links")]
    pub max_links: u64,
    #[serde(default = "default_max_images")]
    pub max_images: u64,
    #[serde(default = "default_workers")]
    pub workers: u64,
}

fn default_max_links() -> u64 { 100 }
fn default_max_images() -> u64 { 100 }
fn default_workers() -> u64 { 4 }

/// Crawl job response
#[derive(Debug, Serialize)]
pub struct CrawlResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

/// Job status
#[derive(Debug, Clone, Serialize)]
pub struct JobStatus {
    pub job_id: String,
    pub url: String,
    pub status: JobState,
    pub pages_crawled: usize,
    pub images_downloaded: usize,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// App state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub jobs: Arc<RwLock<HashMap<String, JobStatus>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Start a new crawl job
async fn start_crawl(
    State(state): State<AppState>,
    Json(req): Json<CrawlRequest>,
) -> Result<Json<CrawlResponse>, StatusCode> {
    let job_id = Uuid::new_v4().to_string();
    
    // Validate URL
    if req.url.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Create job status
    let job = JobStatus {
        job_id: job_id.clone(),
        url: req.url.clone(),
        status: JobState::Running,
        pages_crawled: 0,
        images_downloaded: 0,
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: None,
    };
    
    // Store job
    state.jobs.write().await.insert(job_id.clone(), job);
    
    // TODO: Actually start the crawl in background
    // For now, we'll simulate it
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Simulate crawling
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        // Update job status
        if let Some(job) = state_clone.jobs.write().await.get_mut(&job_id_clone) {
            job.status = JobState::Completed;
            job.pages_crawled = req.max_links as usize;
            job.images_downloaded = req.max_images as usize;
            job.completed_at = Some(chrono::Utc::now().to_rfc3339());
        }
    });
    
    Ok(Json(CrawlResponse {
        job_id,
        status: "started".to_string(),
        message: format!("Crawl job started for {}", req.url),
    }))
}

/// Get job status
async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<JobStatus>, StatusCode> {
    let jobs = state.jobs.read().await;
    
    match jobs.get(&job_id) {
        Some(job) => Ok(Json(job.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// List all jobs
async fn list_jobs(
    State(state): State<AppState>,
) -> Json<Vec<JobStatus>> {
    let jobs = state.jobs.read().await;
    let job_list: Vec<JobStatus> = jobs.values().cloned().collect();
    Json(job_list)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/crawl", post(start_crawl))
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/:job_id", get(get_job_status))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response, "OK");
    }
}
