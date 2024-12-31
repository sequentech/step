.PHONY: docker-image-openwhisk
docker-image-openwhisk:
	docker build --build-arg FEATURES=openwhisk --build-arg DOC_RENDERER_BACKEND=openwhisk --build-arg BASE_IMAGE=alpine:latest -f Dockerfile -t doc_renderer-openwhisk:latest ../..

.PHONY: docker-image-aws-lambda
docker-image-aws-lambda:
	docker build --build-arg FEATURES=aws_lambda --build-arg DOC_RENDERER_BACKEND=aws_lambda --build-arg BASE_IMAGE=alpine:latest -f Dockerfile -t 581718213778.dkr.ecr.us-east-1.amazonaws.com/doc-renderer:latest ../..
