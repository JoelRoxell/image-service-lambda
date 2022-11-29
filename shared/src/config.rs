use crate::image::TransformCfg;
use aws_sdk_appconfig as app_cfg;
use aws_sdk_dynamodb as dynamo_db;
use aws_sdk_s3 as s3;
use lambda_http::Error;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// From CDK deployment
    pub raw_bucket: Option<String>,

    /// From CDK deployment
    pub transformed_bucket: Option<String>,

    /// From CDK deployment
    pub db_table: Option<String>,

    /// From AppConfig, a list that defines default transformation(s) for images uploaded to the raw-bucket.
    pub default_transformations: Vec<TransformCfg>,

    /// From AppConfig, sets time-to-live for s3-pre-signed put url.
    pub upload_ttl: u64,

    /// From AppConfig, use to protect the service endpoints(s).
    pub api_key: String,
}

pub async fn get_s3_client() -> s3::Client {
    let cfg = aws_config::load_from_env().await;

    s3::Client::new(&cfg)
}

pub async fn get_dynamo_client() -> dynamo_db::Client {
    let cfg = aws_config::load_from_env().await;

    dynamo_db::Client::new(&cfg)
}

pub async fn get_service_cfg(app_client: &str) -> Result<Config, Error> {
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
    let configuration = env::var("APP_CONFIGURATION").unwrap();
    let config = aws_config::load_from_env().await;
    let client = app_cfg::Client::new(&config);

    // TODO: read params from env
    let res = client
        .get_configuration()
        .client_id(app_client)
        .environment(&environment)
        .application("image-service")
        .configuration(&configuration)
        .send()
        .await?;
    let raw_string = String::from_utf8(
        res.content()
            .expect("Failed to read content from AppConfig")
            .to_owned()
            .into_inner(),
    )?;
    let mut config: Config = serde_json::from_str(&raw_string)?;
    let raw_bucket = env::var("RAW_BUCKET").unwrap_or_default();
    let transformed_bucket = env::var("TRANSFORMED_BUCKET").unwrap_or_default();
    let table = env::var("DB_TABLE").unwrap_or_default();

    config.raw_bucket = Some(raw_bucket);
    config.transformed_bucket = Some(transformed_bucket);
    config.db_table = Some(table);

    tracing::event!(tracing::Level::DEBUG, "{:?}", config);

    Ok(config)
}
