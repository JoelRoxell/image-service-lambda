import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as s3 from "aws-cdk-lib/aws-s3";
import { RetentionDays } from "aws-cdk-lib/aws-logs";
import * as lambdaEventSources from "aws-cdk-lib/aws-lambda-event-sources";
import { CfnOutput, RemovalPolicy } from "aws-cdk-lib";
import { AttributeType, BillingMode } from "aws-cdk-lib/aws-dynamodb";
import * as cloudfront from "aws-cdk-lib/aws-cloudfront";

export class ImageServiceStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const rawBucket = new s3.Bucket(this, "raw-images", {
      removalPolicy: RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });
    const formattedBucket = new s3.Bucket(this, "formatted-images", {
      removalPolicy: RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });

    const db = new dynamodb.Table(this, "image-service-table", {
      partitionKey: {
        name: "image",
        type: AttributeType.STRING,
      },
      sortKey: {
        name: "cfg",
        type: AttributeType.STRING,
      },
      billingMode: BillingMode.PAY_PER_REQUEST,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    const imageUploadLambda = new lambda.Function(this, "upload-img", {
      functionName: "upload-img",
      code: lambda.Code.fromAsset("../target/lambda/upload-img"),
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "not.required",
      environment: {
        RUST_BACKTRACE: "1",
        RAW_BUCKET: rawBucket.bucketName,
        UPLOAD_TTL: "60",
      },
      logRetention: RetentionDays.ONE_DAY,
    });
    const imageTransformerS3 = new lambda.Function(this, "transform-s3", {
      functionName: "transform-s3",
      code: lambda.Code.fromAsset("../target/lambda/transform-s3"),
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "not.required",
      environment: {
        RUST_BACKTRACE: "1",
        FORMATTED_BUCKET: formattedBucket.bucketName,
        DB_TABLE: db.tableName,
      },
      logRetention: RetentionDays.ONE_DAY,
    });
    const imageTransformer = new lambda.Function(this, "transform-img", {
      functionName: "transform-img",
      code: lambda.Code.fromAsset("../target/lambda/transform-img"),
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "not.required",
      environment: {
        RUST_BACKTRACE: "1",
        RAW_BUCKET: rawBucket.bucketName,
        FORMATTED_BUCKET: formattedBucket.bucketName,
        DB_TABLE: db.tableName,
      },
      logRetention: RetentionDays.ONE_DAY,
    });

    const uploadUrl = new cdk.aws_lambda.CfnUrl(this, "upload-img-url", {
      targetFunctionArn: imageUploadLambda.functionArn,
      authType: cdk.aws_lambda.FunctionUrlAuthType.NONE,
    });

    new cdk.CfnResource(this, "upload-lambda-url-permission", {
      type: "AWS::Lambda::Permission",
      properties: {
        Action: "lambda:InvokeFunctionUrl",
        FunctionName: imageUploadLambda.functionArn,
        Principal: "*",
        FunctionUrlAuthType: "NONE",
      },
    });

    const transformUrl = new cdk.aws_lambda.CfnUrl(this, "transform-img-url", {
      targetFunctionArn: imageTransformer.functionArn,
      authType: cdk.aws_lambda.FunctionUrlAuthType.NONE,
    });

    // TODO: call via cloudfront
    new cdk.CfnResource(this, "transform-img-url-permission", {
      type: "AWS::Lambda::Permission",
      properties: {
        Action: "lambda:InvokeFunctionUrl",
        FunctionName: imageTransformer.functionArn,
        Principal: "*",
        FunctionUrlAuthType: "NONE",
      },
    });

    new cloudfront.CloudFrontWebDistribution(this, "image-service-cloudfront", {
      originConfigs: [
        {
          customOriginSource: {
            domainName: cdk.Fn.select(
              2,
              cdk.Fn.split("/", transformUrl.attrFunctionUrl)
            ),
          },
          behaviors: [
            {
              forwardedValues: {
                queryString: true,
              },
              isDefaultBehavior: true,
            },
          ],
        },
      ],
    });

    const uploadEvent = new lambdaEventSources.S3EventSource(rawBucket, {
      events: [s3.EventType.OBJECT_CREATED],
    });

    imageTransformerS3.addEventSource(uploadEvent);

    rawBucket.grantPut(imageUploadLambda);
    rawBucket.grantRead(imageTransformerS3);
    rawBucket.grantRead(imageTransformer);

    formattedBucket.grantPut(imageTransformerS3);
    formattedBucket.grantReadWrite(imageTransformer);

    db.grantReadWriteData(imageTransformer);
    db.grantReadWriteData(imageTransformerS3);

    new CfnOutput(this, "img-transform-url", {
      value: transformUrl.attrFunctionUrl,
    });
    new CfnOutput(this, "upload-url", {
      value: uploadUrl.attrFunctionUrl,
    });
    new CfnOutput(this, "raw-bucket", {
      value: rawBucket.bucketName,
    });
    new CfnOutput(this, "formatted-bucket", {
      value: formattedBucket.bucketName,
    });
  }
}
