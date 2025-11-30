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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Operation, PathItem, SwaggerSpec};
    use std::collections::HashMap;

    fn create_test_operation(summary: &str, tags: Vec<String>) -> Operation {
        Operation {
            summary: Some(summary.to_string()),
            tags: Some(tags),
            parameters: None,
        }
    }

    #[test]
    fn test_parse_empty_spec() {
        let spec = SwaggerSpec {
            paths: HashMap::new(),
        };
        let endpoints = parse_swagger_spec(spec);
        assert_eq!(endpoints.len(), 0);
    }

    #[test]
    fn test_parse_single_get_endpoint() {
        let mut paths = HashMap::new();
        paths.insert(
            "/users".to_string(),
            PathItem {
                get: Some(create_test_operation(
                    "Get all users",
                    vec!["Users".to_string()],
                )),
                post: None,
                put: None,
                delete: None,
                patch: None,
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].method, "GET");
        assert_eq!(endpoints[0].path, "/users");
        assert_eq!(endpoints[0].summary, Some("Get all users".to_string()));
        assert_eq!(endpoints[0].tags, vec!["Users".to_string()]);
    }

    #[test]
    fn test_parse_all_http_methods() {
        let mut paths = HashMap::new();
        paths.insert(
            "/users".to_string(),
            PathItem {
                get: Some(create_test_operation("Get users", vec![])),
                post: Some(create_test_operation("Create user", vec![])),
                put: Some(create_test_operation("Update user", vec![])),
                delete: Some(create_test_operation("Delete user", vec![])),
                patch: Some(create_test_operation("Patch user", vec![])),
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        assert_eq!(endpoints.len(), 5);

        let methods: Vec<String> = endpoints.iter().map(|e| e.method.clone()).collect();
        assert!(methods.contains(&"GET".to_string()));
        assert!(methods.contains(&"POST".to_string()));
        assert!(methods.contains(&"PUT".to_string()));
        assert!(methods.contains(&"DELETE".to_string()));
        assert!(methods.contains(&"PATCH".to_string()));
    }

    #[test]
    fn test_parse_multiple_paths() {
        let mut paths = HashMap::new();
        paths.insert(
            "/users".to_string(),
            PathItem {
                get: Some(create_test_operation(
                    "Get users",
                    vec!["Users".to_string()],
                )),
                post: None,
                put: None,
                delete: None,
                patch: None,
            },
        );
        paths.insert(
            "/posts".to_string(),
            PathItem {
                get: Some(create_test_operation(
                    "Get posts",
                    vec!["Posts".to_string()],
                )),
                post: Some(create_test_operation(
                    "Create post",
                    vec!["Posts".to_string()],
                )),
                put: None,
                delete: None,
                patch: None,
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        assert_eq!(endpoints.len(), 3);

        let paths_found: Vec<&str> = endpoints.iter().map(|e| e.path.as_str()).collect();
        assert!(paths_found.contains(&"/users"));
        assert!(paths_found.contains(&"/posts"));
    }

    #[test]
    fn test_parse_operation_without_summary() {
        let mut paths = HashMap::new();
        paths.insert(
            "/test".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: None,
                    tags: Some(vec!["Test".to_string()]),
                    parameters: None,
                }),
                post: None,
                put: None,
                delete: None,
                patch: None,
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].summary, None);
    }

    #[test]
    fn test_parse_operation_without_tags() {
        let mut paths = HashMap::new();
        paths.insert(
            "/test".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: Some("Test endpoint".to_string()),
                    tags: None,
                    parameters: None,
                }),
                post: None,
                put: None,
                delete: None,
                patch: None,
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].tags.len(), 0); // Should default to empty vec
    }

    #[test]
    fn test_parse_with_parameters() {
        use crate::types::ApiParameter;

        let mut paths = HashMap::new();
        paths.insert(
            "/users/{id}".to_string(),
            PathItem {
                get: Some(Operation {
                    summary: Some("Get user by ID".to_string()),
                    tags: Some(vec!["Users".to_string()]),
                    parameters: Some(vec![ApiParameter {
                        name: "id".to_string(),
                        location: "path".to_string(),
                        required: Some(true),
                        schema: None,
                        description: Some("User ID".to_string()),
                    }]),
                }),
                post: None,
                put: None,
                delete: None,
                patch: None,
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].parameters.len(), 1);
        assert_eq!(endpoints[0].parameters[0].name, "id");
        assert_eq!(endpoints[0].parameters[0].location, "path");
    }

    #[test]
    fn test_parse_empty_operations_ignored() {
        let mut paths = HashMap::new();
        paths.insert(
            "/users".to_string(),
            PathItem {
                get: None,
                post: None,
                put: None,
                delete: None,
                patch: None,
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        // No operations defined, so no endpoints should be created
        assert_eq!(endpoints.len(), 0);
    }

    #[test]
    fn test_parse_multiple_tags() {
        let mut paths = HashMap::new();
        paths.insert(
            "/admin/users".to_string(),
            PathItem {
                get: Some(create_test_operation(
                    "Admin users",
                    vec!["Admin".to_string(), "Users".to_string()],
                )),
                post: None,
                put: None,
                delete: None,
                patch: None,
            },
        );

        let spec = SwaggerSpec { paths };
        let endpoints = parse_swagger_spec(spec);

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].tags.len(), 2);
        assert!(endpoints[0].tags.contains(&"Admin".to_string()));
        assert!(endpoints[0].tags.contains(&"Users".to_string()));
    }
}
