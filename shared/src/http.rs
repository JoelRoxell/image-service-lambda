use aws_lambda_events::http::StatusCode;
use lambda_http::{Body, Request, Response};

use lambda_http::Error;

use crate::config::Config;

pub fn is_authorized(event: &Request, cfg: &Config) -> Result<(), Error> {
    let key = event.headers().get("x-api-key");

    match key.is_some() && cfg.api_key.eq(key.unwrap()) {
        true => Ok(()),
        false => Err(Error::from("Unauthorized")),
    }
}

pub fn unauthorized_request() -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Body::Empty)?)
}

pub fn bad_request() -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::Empty)?)
}

pub fn create_response(status: StatusCode, body: Body) -> Result<Response<Body>, Error> {
    Ok(Response::builder().status(status).body(body)?)
}
