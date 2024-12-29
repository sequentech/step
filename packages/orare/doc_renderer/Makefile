.PHONY: docker-image-openwhisk
docker-image-openwhisk:
	docker build --build-arg FEATURES=openwhisk --build-arg BASE_IMAGE=alpine:latest -f Dockerfile -t doc_renderer-openwhisk:latest ../..

.PHONY: docker-image-aws-lambda
docker-image-aws-lambda:
	docker build --build-arg FEATURES=aws_lambda --build-arg BASE_IMAGE=alpine:latest -f Dockerfile -t doc_renderer-openwhisk:latest ../..
