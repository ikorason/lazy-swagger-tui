use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
pub struct SwaggerSpec {
    pub paths: HashMap<String, PathItem>,
}

#[derive(Deserialize)]
pub struct PathItem {
    pub get: Option<Operation>,
    pub post: Option<Operation>,
    pub put: Option<Operation>,
    pub delete: Option<Operation>,
    pub patch: Option<Operation>,
}

#[derive(Deserialize)]
pub struct Operation {
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Flat,
    Grouped,
}

#[derive(Debug, Clone)]
pub enum LoadingState {
    Idle,
    Fetching,
    Parsing,
    Complete,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum RenderItem {
    GroupHeader {
        name: String,
        count: usize,
        expanded: bool,
    },
    Endpoint {
        endpoint: ApiEndpoint,
    },
}
