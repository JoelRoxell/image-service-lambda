# Serverless Images Service [WIP]

Maintenance free serverless image-service written in Rust.

> Lambda, S3, sns, Dynamo DB, CloudFront.

Features

- [ ] Docs (envs, configs, deployment, stages)
- [x] Upload images to s3 from a client
- [x] Rescale on demand
- [x] Secure by api-secret
- [x] Read pre-scaled img cfgs from s3 or app cfg
- [x] Get image
- [x] Pre-scale based on list of cfg
- [ ] Trigger event on SNS - consumer should count total images that has ever been transformed.  
- [x] Move to lambda url(s)
- [x] CloudFront
- [x] Move everything to one folder and produce multiple bin(s)
- [ ] SQS?
- [ ] Delete img + generated formats
- [ ] Cleanup
- [ ] Image TTLs (dynamo, s3)
- [ ] Backup strategy
- [ ] create a presigned fetch url if payload (img) is > 6mb

## Usage

```bash
curl {api}/images # outputs upload uri   

curl -X PUT {upload-uri} --upload-file  transformer/res/test-img.png

curl  {api}/images/{id}\?h\=200\&w\=200
```
