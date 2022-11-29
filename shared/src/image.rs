use aws_sdk_dynamodb as dynamo_db;
use aws_sdk_s3 as s3;
use aws_smithy_http::body::SdkBody;
use dynamo_db::model::AttributeValue;
use image::imageops;
use image::io::Reader;
use lambda_runtime::Error;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::io::{self, Cursor};
use tokio::fs;

pub async fn transform_img(
    raw_bucket: &str,
    transformed_bucket: &str,
    obj: &str,
    transformations: &Vec<TransformCfg>,
    s3: &s3::Client,
    dynamo_db: &dynamo_db::Client,
    db_table: &str,
) -> Result<Vec<String>, Error> {
    let cmd_output = s3.get_object().bucket(raw_bucket).key(obj).send().await?;
    let original_img = cmd_output
        .body
        .collect()
        .await
        .map(|data| data.into_bytes())?;

    let mut generated_images = vec![];

    for cfg in transformations {
        let img_hash = get_sha1(obj, cfg);
        let transform_query = dynamo_db
            .query()
            .table_name(db_table.to_owned())
            .key_condition_expression("image = :pk and cfg = :sort")
            .expression_attribute_values(":pk", AttributeValue::S(obj.to_string()))
            .expression_attribute_values(":sort", AttributeValue::S(img_hash.clone()))
            .send()
            .await?;

        if transform_query.count() > 0 {
            generated_images.push(img_hash);
            // Skip transform if it already exists
            continue;
        }

        let new_img_hash = transform(obj, original_img.to_vec(), *cfg)
            .await
            .expect("Failed to transform original image");

        // TODO: remove tmp img, couldn't stream the img and had to tmp store it on disk after transformation...
        let transformed_img = fs::read(format!("/tmp/{}", &new_img_hash)).await?;

        s3.put_object()
            .bucket(transformed_bucket.to_owned())
            .key(&new_img_hash)
            .body(s3::types::ByteStream::new(SdkBody::from(transformed_img)))
            .content_type("image/png")
            .send()
            .await?;
        dynamo_db
            .put_item()
            .table_name(db_table.to_owned())
            .item(
                "image",
                dynamo_db::model::AttributeValue::S(obj.to_string()),
            )
            .item(
                "cfg",
                dynamo_db::model::AttributeValue::S(new_img_hash.clone()),
            )
            .send()
            .await?;

        generated_images.push(new_img_hash);
    }

    Ok(generated_images)
}

async fn transform(
    filename: &str,
    bytes: Vec<u8>,
    cfg: TransformCfg,
) -> Result<String, Box<dyn std::error::Error>> {
    let img_reader = Reader::new(Cursor::new(bytes));
    let img_reader = img_reader.with_guessed_format()?;
    let og_img = img_reader.decode()?;
    let hash = get_sha1(filename, &cfg);

    let transformed_img = og_img.resize(cfg.width, cfg.height, imageops::FilterType::Gaussian);

    let tmp_img = fs::File::create(format!("/tmp/{}", &hash))
        .await
        .unwrap()
        .into_std()
        .await;
    let mut writer = io::BufWriter::new(tmp_img);

    transformed_img.write_to(&mut writer, image::ImageOutputFormat::Png)?;

    Ok(hash)
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TransformCfg {
    // Add more transformation options here...
    pub width: u32,
    pub height: u32,
}

impl TransformCfg {
    pub fn new(w: u32, h: u32) -> Self {
        TransformCfg {
            width: w,
            height: h,
        }
    }

    pub fn digest(self) -> String {
        format!("{}{}", self.height, self.width)
    }
}

pub fn get_sha1(name: &str, cfg: &TransformCfg) -> String {
    let mut hash = Sha1::new();

    hash.update(name);
    hash.update(cfg.digest());

    format!("{:X}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::{transform, TransformCfg};
    use sha1::{Digest, Sha1};
    use std::{fs, io::Read, path::Path};

    #[tokio::test]
    async fn image_transform() {
        let path = Path::new("res/test-img.png");
        let mut file = fs::File::open(path).unwrap();
        let mut data = Vec::new();

        file.read_to_end(&mut data).expect("Unable to read data");

        let res = transform("test-file", data, TransformCfg::new(800, 800))
            .await
            .unwrap();

        println!("{:?}", res);
    }

    #[test]
    fn name_cfg_hash() {
        let mut hasher = Sha1::new();
        let cfg = TransformCfg::new(255, 255);

        hasher.update(cfg.digest());

        println!("{:X}", hasher.finalize());
    }
}
