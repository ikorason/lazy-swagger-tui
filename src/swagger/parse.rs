use crate::types::{ApiEndpoint, SwaggerSpec};

pub fn parse_swagger_spec(spec: SwaggerSpec) -> Vec<ApiEndpoint> {
    let mut endpoints: Vec<ApiEndpoint> = Vec::new();

    for (path, path_item) in spec.paths {
        if let Some(op) = &path_item.get {
            endpoints.push(ApiEndpoint {
                method: "GET".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
                parameters: op.parameters.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.post {
            endpoints.push(ApiEndpoint {
                method: "POST".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
                parameters: op.parameters.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.put {
            endpoints.push(ApiEndpoint {
                method: "PUT".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
                parameters: op.parameters.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.delete {
            endpoints.push(ApiEndpoint {
                method: "DELETE".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
                parameters: op.parameters.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.patch {
            endpoints.push(ApiEndpoint {
                method: "PATCH".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
                parameters: op.parameters.clone().unwrap_or_default(),
            });
        }
    }

    endpoints
}
