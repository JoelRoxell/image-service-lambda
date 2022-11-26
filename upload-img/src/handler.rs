use aws_sdk_s3::presigning::config::PresigningConfig;
use lambda_http::{Body, Error, Request, Response};
use serde_json::json;
use std::env;
use std::time::Duration;
use uuid::Uuid;

pub async fn get_upload_uri(_event: Request) -> Result<Response<Body>, Error> {
    let target = create_presigned_url().await?;
    let resp = Response::builder()
        .status(200)
        .body(target.into())
        .map_err(Box::new)?;

    Ok(resp)
}

pub async fn create_presigned_url() -> Result<String, Error> {
    let target = env::var("RAW_BUCKET").expect("failed to read target bucket from env");
    let upload_ttl = env::var("UPLOAD_TTL")
        .expect("failed to read upload-ttl from env")
        .parse::<u64>()
        .expect("failed to parse upload-ttl to u64");

    // TODO: shared init cfg
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);

    let filename = Uuid::new_v4().to_string();
    let presigned_url = client
        .put_object()
        .bucket(&target)
        .key(&filename)
        .presigned(PresigningConfig::expires_in(Duration::new(upload_ttl, 0))?)
        .await?;

    Ok(json!({
       "target": presigned_url.uri().to_string(),
       "filename": filename
    })
    .to_string())
}
