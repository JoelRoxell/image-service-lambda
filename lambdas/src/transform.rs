use aws_sdk_dynamodb as dynamo_db;
use aws_sdk_s3 as s3;
use lambda_http::{http::StatusCode, run, service_fn, Body, Error, Request, RequestExt, Response};
use shared::{
    config::{get_dynamo_client, get_s3_client},
    http::{create_response, is_authorized, unauthorized_request},
    image::TransformCfg,
};
use tracing::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let s3 = get_s3_client().await;
    let dynamo = get_dynamo_client().await;
    let cfg = shared::config::get_service_cfg("transform").await?;

    run(service_fn(|req| transform_img(req, &s3, &dynamo, &cfg))).await
}

pub async fn transform_img(
    req: Request,
    s3: &s3::Client,
    dynamo: &dynamo_db::Client,
    cfg: &shared::config::Config,
) -> Result<Response<Body>, Error> {
    if is_authorized(&req, cfg).is_err() {
        return unauthorized_request();
    }

    let query_params = req.query_string_parameters();
    let params = req
        .uri()
        .path()
        .split('/')
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let img_id = params.get(1).unwrap();
    let height = query_params.first("height").expect("read height").parse()?;
    let width = query_params.first("width").expect("read width").parse()?;

    let transformed_bucket = cfg.transformed_bucket.clone().unwrap();
    let raw_bucket = cfg.transformed_bucket.to_owned().unwrap();
    let transformation_cfg = TransformCfg { height, width };

    let get_formatted_img_cmd = s3
        .get_object()
        .bucket(&transformed_bucket)
        .key(shared::image::get_sha1(img_id, &transformation_cfg))
        .send()
        .await;

    //TODO: redirect if > 6mb
    if let Ok(res) = get_formatted_img_cmd {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "image/png")
            .body(res.body.collect().await?.into_bytes().to_vec().into())?);
    }

    let hash = shared::image::transform_img(
        &raw_bucket,
        &transformed_bucket,
        img_id,
        &vec![transformation_cfg],
        s3,
        dynamo,
        &cfg.db_table
            .clone()
            .expect("failed to read db_table from service cfg"),
    )
    .await?
    .first()
    .expect("Failed to transform img")
    .to_owned();

    let cmd_res = s3
        .get_object()
        .bucket(transformed_bucket)
        .key(&hash)
        .send()
        .await;

    // TODO:: create a presigned fetch url if payload is > 6mb
    let resp = if let Ok(cmd_res) = cmd_res {
        let img_data = cmd_res.body.collect().await?.into_bytes().to_vec().into();

        Response::builder()
            .status(StatusCode::CREATED)
            .header("content-type", "image/png")
            .body(img_data)?
    } else {
        event!(
            tracing::Level::ERROR,
            "failed to read {} from formatted-bucket={}",
            hash,
            cfg.transformed_bucket
                .clone()
                .unwrap_or_else(|| "n/a".to_string())
        );
        event!(tracing::Level::ERROR, "{:?}", cmd_res.err());

        create_response(StatusCode::INTERNAL_SERVER_ERROR, Body::Empty)?
    };

    Ok(resp)
}
