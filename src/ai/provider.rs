//! AI Provider abstraction for BYOK support
#![allow(dead_code)]

use async_trait::async_trait;
use futures::Stream;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AIError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Rate limited")]
    RateLimited,
}

#[derive(Debug, Clone)]
pub struct CompletionOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// Trait for AI providers - allows BYOK
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Get a completion from the AI
    async fn complete(
        &self,
        prompt: &str,
        opts: Option<CompletionOptions>,
    ) -> Result<String, AIError>;

    /// Stream a completion (for better UX)
    async fn stream(
        &self,
        prompt: &str,
        opts: Option<CompletionOptions>,
    ) -> Result<impl Stream<Item = Result<String, AIError>>, AIError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // AIError Tests
    // =============================================================================

    #[test]
    fn test_ai_error_api_variant() {
        let err = AIError::Api("Request failed".to_string());
        assert_eq!(format!("{}", err), "API error: Request failed");
    }

    #[test]
    fn test_ai_error_config_variant() {
        let err = AIError::Config("Invalid key".to_string());
        assert_eq!(format!("{}", err), "Configuration error: Invalid key");
    }

    #[test]
    fn test_ai_error_rate_limited_variant() {
        let err = AIError::RateLimited;
        assert_eq!(format!("{}", err), "Rate limited");
    }

    #[test]
    fn test_ai_error_debug_impl() {
        let err = AIError::Api("test".to_string());
        // Debug should show the variant name
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Api"));
    }

    #[test]
    fn test_ai_error_is_std_error() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<AIError>();
    }

    #[test]
    fn test_ai_error_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AIError>();
    }

    // =============================================================================
    // CompletionOptions Tests
    // =============================================================================

    #[test]
    fn test_completion_options_default() {
        let opts = CompletionOptions {
            max_tokens: None,
            temperature: None,
        };
        assert!(opts.max_tokens.is_none());
        assert!(opts.temperature.is_none());
    }

    #[test]
    fn test_completion_options_with_values() {
        let opts = CompletionOptions {
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        assert_eq!(opts.max_tokens, Some(100));
        assert_eq!(opts.temperature, Some(0.7));
    }

    #[test]
    fn test_completion_options_clone() {
        let opts = CompletionOptions {
            max_tokens: Some(50),
            temperature: Some(0.5),
        };
        let cloned = opts.clone();
        assert_eq!(opts.max_tokens, cloned.max_tokens);
        assert_eq!(opts.temperature, cloned.temperature);
    }

    #[test]
    fn test_completion_options_debug() {
        let opts = CompletionOptions {
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        let debug_str = format!("{:?}", opts);
        assert!(debug_str.contains("CompletionOptions"));
        assert!(debug_str.contains("max_tokens"));
        assert!(debug_str.contains("temperature"));
    }

    // =============================================================================
    // Mock Provider for Testing
    // =============================================================================

    /// A mock AI provider for testing purposes
    struct MockProvider {
        response: Result<String, AIError>,
    }

    impl MockProvider {
        fn new(response: Result<String, AIError>) -> Self {
            Self { response }
        }

        fn with_success(response: &str) -> Self {
            Self {
                response: Ok(response.to_string()),
            }
        }

        fn with_error(err: AIError) -> Self {
            Self { response: Err(err) }
        }
    }

    #[async_trait]
    impl AIProvider for MockProvider {
        async fn complete(
            &self,
            _prompt: &str,
            _opts: Option<CompletionOptions>,
        ) -> Result<String, AIError> {
            self.response.clone()
        }

        async fn stream(
            &self,
            prompt: &str,
            opts: Option<CompletionOptions>,
        ) -> Result<impl Stream<Item = Result<String, AIError>>, AIError> {
            // Get the completion first
            let result = self.complete(prompt, opts).await?;
            // Return as a single-item stream
            Ok(futures::stream::once(async move { Ok(result) }))
        }
    }

    // =============================================================================
    // Mock Provider Tests
    // =============================================================================

    #[tokio::test]
    async fn test_mock_provider_success() {
        let provider = MockProvider::with_success("Hello, world!");
        let result = provider.complete("test prompt", None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }

    #[tokio::test]
    async fn test_mock_provider_api_error() {
        let provider = MockProvider::with_error(AIError::Api("Connection failed".to_string()));
        let result = provider.complete("test prompt", None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AIError::Api(_)));
        assert!(format!("{}", err).contains("Connection failed"));
    }

    #[tokio::test]
    async fn test_mock_provider_config_error() {
        let provider = MockProvider::with_error(AIError::Config("Invalid config".to_string()));
        let result = provider.complete("test prompt", None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AIError::Config(_)));
    }

    #[tokio::test]
    async fn test_mock_provider_rate_limited_error() {
        let provider = MockProvider::with_error(AIError::RateLimited);
        let result = provider.complete("test prompt", None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AIError::RateLimited));
    }

    #[tokio::test]
    async fn test_mock_provider_with_options() {
        let provider = MockProvider::with_success("Response");
        let opts = CompletionOptions {
            max_tokens: Some(100),
            temperature: Some(0.5),
        };
        let result = provider.complete("test", Some(opts)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_provider_stream_success() {
        let provider = MockProvider::with_success("Streaming response");
        let stream = provider.stream("test prompt", None).await;
        assert!(stream.is_ok());

        // Box the stream to make it Unpin
        let stream = Box::pin(stream.unwrap());
        let mut stream = stream;
        use futures::StreamExt;
        let item = stream.next().await;
        assert!(item.is_some());
        let result = item.unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Streaming response");
    }

    #[tokio::test]
    async fn test_mock_provider_stream_error() {
        let provider = MockProvider::with_error(AIError::RateLimited);
        let result = provider.stream("test prompt", None).await;
        assert!(result.is_err());
    }

    // =============================================================================
    // Provider Trait Object Tests
    // =============================================================================

    #[test]
    fn test_provider_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        // This tests that MockProvider (and by extension AIProvider implementors)
        // can be used in contexts requiring Send + Sync
        assert_send_sync::<MockProvider>();
    }
}
