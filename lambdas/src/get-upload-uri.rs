use aws_lambda_events::http::StatusCode;
use aws_sdk_s3::{presigning::config::PresigningConfig, Client};
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde_json::json;
use shared::http::{bad_request, is_authorized};
use std::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .without_time()
        .init();

    let config = aws_config::load_from_env().await;
    let s3 = aws_sdk_s3::Client::new(&config);
    let cfg = shared::config::get_service_cfg("get-upload-uri").await?;

    run(service_fn(|req| get_upload_uri(req, &s3, &cfg))).await
}

pub async fn get_upload_uri(
    req: Request,
    s3: &Client,
    cfg: &shared::config::Config,
) -> Result<Response<Body>, Error> {
    if is_authorized(&req, cfg).is_err() {
        return bad_request();
    };

    let filename = Uuid::new_v4().to_string();
    let presigned_url = s3
        .put_object()
        .bucket(
            cfg.raw_bucket
                .clone()
                .expect("failed to read raw_bucket from cfg"),
        )
        .key(&filename)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(
            cfg.upload_ttl,
        ))?)
        .await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(
            json!({
               "target": presigned_url.uri().to_string(),
               "filename": filename
            })
            .to_string()
            .into(),
        )
        .map_err(Box::new)?)
}
