#!/bin/bash
mc config host add myminio $MINIO_PRIVATE_URI $MINIO_ROOT_USER $MINIO_ROOT_PASSWORD;
mc mb -p myminio/$MINIO_PUBLIC_BUCKET;
mc mb -p myminio/$MINIO_BUCKET;
mc anonymous set download myminio/$MINIO_PUBLIC_BUCKET;
mc admin user svcacct add --access-key $MINIO_ACCESS_KEY --secret-key $MINIO_ACCESS_SECRET myminio $MINIO_ROOT_USER;
exit 0;
