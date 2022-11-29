build:
	cargo lambda build --release

deploy: build
	cd infrastructure && npm run cdk deploy && cd -- 

destroy:
	cd infrastructure && npm run cdk destroy && cd -- 

