build:
	cargo lambda build --release

deploy: build
	cd infra && npm run cdk deploy && cd -- 

destroy:
	cd infra && npm run cdk destroy && cd -- 

