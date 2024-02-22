#!/bin/bash
mc config host add myminio $MINIO_PRIVATE_URI $MINIO_ROOT_USER $MINIO_ROOT_PASSWORD;
mc mb -p myminio/$MINIO_PUBLIC_BUCKET;
mc mb -p myminio/$MINIO_BUCKET;
mc anonymous set download myminio/$MINIO_PUBLIC_BUCKET;
mc admin user svcacct add --access-key $MINIO_ACCESS_KEY --secret-key $MINIO_ACCESS_SECRET myminio $MINIO_ROOT_USER;
mc stat myminio/public/certs.json

if [ $? -eq 1 ]; then
    echo uploading file
    mc cp /scripts/certs.json myminio/public/certs.json
else
    echo file already exists
fi


FOLDER_PATH="./public-assets" 
DESTINATION="myminio/$MINIO_PUBLIC_BUCKET/public-assets" 
echo "Uploading folder $FOLDER_PATH to $DESTINATION"
mc cp -r $FOLDER_PATH $DESTINATION

exit 0;
