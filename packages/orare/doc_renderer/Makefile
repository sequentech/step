.PHONY: docker-image-openwhisk
docker-image-openwhisk:
	docker build --progress=plain --network=host --build-arg FEATURES=openwhisk --build-arg BASE_IMAGE=alpine:3.17 -f Dockerfile -t ${REGISTRY}/doc_renderer:latest ../..

.PHONY: docker-image-aws-lambda
docker-image-aws-lambda:
# headless_chrome is very brittle --especially on AWS Lambda--: alpine
# 3.17 contains chromium 112 that is working as expected with
# headless_chrome.
	docker buildx build --progress=plain --provenance=false --network=host --build-arg FEATURES=aws_lambda --build-arg BASE_IMAGE=alpine:3.17 -f Dockerfile -t 581718213778.dkr.ecr.us-east-1.amazonaws.com/doc-renderer:latest ../..
