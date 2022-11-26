use image::imageops;
use image::io::Reader;
use sha1::{Digest, Sha1};
use std::error::Error;
use std::io::Cursor;
use std::{fs, io};

#[derive(Debug)]
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

pub fn transform(
    filename: &str,
    bytes: Vec<u8>,
    cfg: TransformCfg,
) -> Result<String, Box<dyn Error>> {
    let img_reader = Reader::new(Cursor::new(bytes));
    let img_reader = img_reader.with_guessed_format()?;
    let og_img = img_reader.decode()?;
    let transformed_img = og_img.resize(cfg.width, cfg.height, imageops::FilterType::Gaussian);

    let mut hash = Sha1::new();

    hash.update(filename);
    hash.update(cfg.digest());

    let hash = format!("{:X}", hash.finalize());

    let tmp_img = fs::File::create(format!("/tmp/{}", &hash)).unwrap();
    let mut writer = io::BufWriter::new(tmp_img);

    transformed_img.write_to(&mut writer, image::ImageOutputFormat::Png)?;

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::{transform, TransformCfg};
    use sha1::{Digest, Sha1};
    use std::{fs, io::Read, path::Path};

    #[test]
    fn image_transform() {
        let path = Path::new("res/test-img.png");
        let mut file = fs::File::open(path).unwrap();
        let mut data = Vec::new();

        file.read_to_end(&mut data).expect("Unable to read data");

        let res = transform("test-file", data, TransformCfg::new(800, 800)).unwrap();

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
