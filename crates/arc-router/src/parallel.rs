use futures::future::select_ok;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use futures::stream::BoxStream;

use arc_providers::traits::Provider;
use arc_providers::message::{Message, ToolDefinition, StreamEvent};

/// Race multiple streaming providers simultaneously — return first successful response stream
pub async fn race_providers(
    providers: &[Box<dyn Provider>],
    messages: &[Message],
    tools: &[ToolDefinition],
    model_overrides: &HashMap<String, String>,
    deadline: Duration,
) -> anyhow::Result<(BoxStream<'static, Result<StreamEvent, anyhow::Error>>, String, Duration)> {
    
    // We create a vector of futures wrapped in Box::pin for select_ok
    let futures = providers.iter().map(|p| {
        let model = model_overrides
            .get(p.name())
            .cloned()
            .unwrap_or_else(|| "default".to_string());
            
        let msgs = messages.to_vec();
        let tls = tools.to_vec();

        Box::pin(async move {
            let start = Instant::now();
            let result = timeout(deadline, p.stream(&model, &msgs, &tls)).await;
            let latency = start.elapsed();

            match result {
                Ok(Ok(stream)) => Ok((stream, p.name().to_string(), latency)),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(anyhow::anyhow!("Timeout: {}", e)),
            }
        })
    });

    // select_ok semantics — first success wins, cancel the rest immediately
    let (result, _remaining_futures) = select_ok(futures).await?;
    
    Ok(result)
}
