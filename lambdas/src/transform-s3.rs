use aws_lambda_events::event::s3::S3Event;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let s3 = shared::config::get_s3_client().await;
    let dynamo = shared::config::get_dynamo_client().await;
    let transformed_bucket = env::var("FORMATTED_BUCKET")?;
    let table = env::var("DYNAMO_TABLE")?;
    let cfg = shared::config::get_service_cfg("transform-s3").await?;

    run(service_fn(|event| {
        transform_s3(event, &s3, &dynamo, &transformed_bucket, &table, &cfg)
    }))
    .await
}

async fn transform_s3(
    event: LambdaEvent<S3Event>,
    s3: &aws_sdk_s3::Client,
    dynamo: &aws_sdk_dynamodb::Client,
    transformed_bucket: &str,
    table: &str,
    cfg: &shared::config::Config,
) -> Result<(), Error> {
    for record in event.payload.records {
        let s3_target = record.s3.clone();
        let raw_bucket = s3_target
            .bucket
            .name
            .expect("Failed to read bucket name from event");
        let obj = s3_target
            .object
            .key
            .expect("Failed to read object key from event");

        shared::image::transform_img(
            &raw_bucket,
            transformed_bucket,
            &obj,
            &cfg.default_transformations,
            s3,
            dynamo,
            table,
        )
        .await?;
    }

    Ok(())
}
