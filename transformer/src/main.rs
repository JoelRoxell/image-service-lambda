use aws_lambda_events::event::s3::S3Event;
use aws_sdk_s3::{types::ByteStream, Client};
use aws_smithy_http::body::SdkBody;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use std::env;
use tokio::fs;
use transform::TransformCfg;

mod transform;

async fn function_handler(event: LambdaEvent<S3Event>) -> Result<(), Error> {
    for record in event.payload.records {
        let s3_target = record.s3.clone();
        let bucket = s3_target
            .bucket
            .name
            .expect("Failed to read bucket name from event");
        let obj = s3_target
            .object
            .key
            .expect("Failed to read object key from event");

        // generate for each predefined size
        transform_img(&bucket, &obj, vec![TransformCfg::new(500, 500)]).await?
    }

    Ok(())
}

pub async fn transform_img(
    bucket: &str,
    obj: &str,
    transform_cfgs: Vec<TransformCfg>,
) -> Result<(), Error> {
    // TODO: shared client config creation
    let transformed_bucket =
        env::var("FORMATTED_BUCKET").expect("Couldn't read target transform bucket from env");
    let cfg = aws_config::load_from_env().await;
    let client = Client::new(&cfg);
    // -----------------------------------

    let cmd_output = client.get_object().bucket(bucket).key(obj).send().await?;
    let original_img = cmd_output
        .body
        .collect()
        .await
        .map(|data| data.into_bytes())?;

    for cfg in transform_cfgs {
        let hash = transform::transform(obj, original_img.to_vec(), cfg)
            .expect("Failed to transform original image");
        let transformed_img = fs::read(format!("/tmp/{}", hash)).await?;

        client
            .put_object()
            .bucket(&transformed_bucket)
            .key(hash)
            .body(ByteStream::new(SdkBody::from(transformed_img)))
            .content_type("image/png")
            .send()
            .await?;

        // TODO: write img url to dynamo
        // pk OG hash sk transformed
        // GET img/123?w=200&h=200 -> hash -> abc
        // check in dynamo if abc exist, return, otherwise transform and return
        // cache req/res
    }

    Ok(())
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
