#[cfg(feature = "async")]
use error_forge::{AppError, AsyncForgeError, AsyncResult};
#[cfg(feature = "async")]
use std::time::Duration;

#[cfg(feature = "async")]
async fn fetch_data(endpoint: &str) -> AsyncResult<String, AppError> {
    // Simulate an async operation like a network request
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    if endpoint.contains("invalid") {
        return Err(AppError::network(endpoint, None::<Box<dyn std::error::Error + Send + Sync>>)
            .with_retryable(true)
            .with_status(503));
    }
    
    Ok(format!("Data from {}", endpoint))
}

#[cfg(feature = "async")]
async fn process_data_with_retry(endpoint: &str, max_retries: usize) -> AsyncResult<String, AppError> {
    let mut retries = 0;
    let mut backoff = 100; // Start with 100ms
    
    loop {
        let result = fetch_data(endpoint).await;
        
        match result {
            Ok(data) => return Ok(data),
            Err(err) if err.is_retryable() && retries < max_retries => {
                // Log the error but continue with retry
                println!("Error occurred: {}. Retrying in {}ms...", err, backoff);
                
                // Use the async_handle method to perform any error-specific recovery
                let _ = err.handle_async().await;
                
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                retries += 1;
                backoff *= 2; // Exponential backoff
            },
            Err(err) => {
                // No more retries or error is not retryable
                return Err(err);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "async")]
    {
        // Create a tokio runtime for our async code
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            println!("Testing async error handling with retry logic:");
            
            // This should succeed
            match process_data_with_retry("api.example.com/users", 3).await {
                Ok(data) => println!("Success: {}", data),
                Err(e) => println!("Failed: {}", e),
            }
            
            // This should fail after retries
            match process_data_with_retry("api.example.com/invalid", 3).await {
                Ok(data) => println!("Success: {}", data),
                Err(e) => println!("Failed after retries: {}", e),
            }
        });
    }
    
    #[cfg(not(feature = "async"))]
    {
        println!("Async feature is not enabled. Run with --features=async to see this example.");
    }
    
    Ok(())
}
