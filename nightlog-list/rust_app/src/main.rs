use futures::TryStreamExt;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use nightlog_common::{log_listing, mongodb_connection, GetListRequest, Log};
use serde::Serialize;

/// Requests come into the runtime as unicode
/// strings in json format, which can map to any structure that implements `serde::Deserialize`
/// The runtime pays no attention to the contents of the request payload.

/// This is a made-up example of what a response structure may look like.
/// There is no restriction on what it can be. The runtime requires responses
/// to be serialized into json. The runtime pays no attention
/// to the contents of the response payload.
#[allow(non_snake_case)]
#[derive(Serialize)]
struct Response {
    statusCode: i32,
    body: String,
}

/// This is the main body for the function.
async fn function_handler(event: LambdaEvent<GetListRequest>) -> Result<Response, Error> {
    let mongodb_client = mongodb_connection().await?;
    let list_req = event.payload;
    let res = log_listing(&mongodb_client, &list_req).await?;
    let body;
    match res.try_collect::<Vec<Log>>().await {
        Ok(vector) => body = serde_json::to_string(&vector)?,
        Err(e) => {
            return Err(format!(
                "an error occurred in collecting user's logs in a vector: {}",
                e
            )
            .into());
        }
    }
    // Prepare the response
    let resp = Response {
        statusCode: 200,
        body,
    };

    // Return `Response` (it will be serialized to JSON automatically by the runtime)
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}

#[cfg(test)]
mod tests {}
