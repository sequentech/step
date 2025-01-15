.PHONY: docker-image-openwhisk
docker-image-openwhisk:
# headless_chrome is very brittle: alpine 3.17 contains chromium 112
# that is working as expected with headless_chrome.
	docker build --progress=plain --network=host --build-arg FEATURES=openwhisk --build-arg BASE_IMAGE=${ALPINE_LAMBDA_BASE_IMAGE} -f Dockerfile -t ${REGISTRY}/doc_renderer:latest ../..

.PHONY: docker-image-aws-lambda
docker-image-aws-lambda:
# headless_chrome is very brittle: alpine 3.17 contains chromium 112
# that is working as expected with headless_chrome.
	docker buildx build --progress=plain --provenance=false --network=host --build-arg FEATURES=aws_lambda --build-arg BASE_IMAGE=${ALPINE_LAMBDA_BASE_IMAGE} -f Dockerfile -t 581718213778.dkr.ecr.us-east-1.amazonaws.com/doc-renderer:latest ../..
