#!/bin/bash

# This is a placeholder script for provisioning a new server on a cloud provider.
# You will need to replace the contents of this script with the actual commands
# for your chosen cloud provider.

################################################################################
# RECOMMENDED MACHINE SPECIFICATIONS
################################################################################
#
# For running the FULL docker-compose profile with all services:
#
# MINIMUM RECOMMENDED:
#   - CPU: 8 vCPUs
#   - RAM: 16 GB
#   - Storage: 50-100 GB SSD
#   - OS: Ubuntu 22.04 LTS or 24.04 LTS
#
# RECOMMENDED INSTANCE TYPES BY PROVIDER:
#   AWS:       c5.2xlarge  (8 vCPUs, 16 GB RAM) - Compute optimized
#              t3.2xlarge  (8 vCPUs, 32 GB RAM) - General purpose with burst
#              m5.2xlarge  (8 vCPUs, 32 GB RAM) - General purpose
#
#   GCP:       n2-standard-8   (8 vCPUs, 32 GB RAM)
#              c2-standard-8   (8 vCPUs, 32 GB RAM) - Compute optimized
#
#   Azure:     Standard_D8s_v3 (8 vCPUs, 32 GB RAM)
#              Standard_F8s_v2 (8 vCPUs, 16 GB RAM) - Compute optimized
#
# NOTE: For base profile only (without admin-portal, voting-portal, trustees),
#       you can use smaller instances (4 vCPUs, 8-16 GB RAM), but this is not
#       recommended for production use.
#
################################################################################

# Below are links to tutorials for creating a new virtual machine instance on
# popular cloud providers:

# Google Cloud Platform (GCP):
# https://cloud.google.com/compute/docs/instances/create-start-instance

# Amazon Web Services (AWS):
# https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/EC2_GetStarted.html

# Microsoft Azure:
# https://docs.microsoft.com/en-us/azure/virtual-machines/linux/quick-create-cli

################################################################################
# EXAMPLE COMMANDS
################################################################################

# Example using AWS CLI to create a c5.2xlarge instance:
# aws ec2 run-instances \
#   --image-id ami-0c55b159cbfafe1f0 \
#   --instance-type c5.2xlarge \
#   --key-name your-key-pair \
#   --security-group-ids sg-xxxxxxxx \
#   --subnet-id subnet-xxxxxxxx \
#   --block-device-mappings '[{"DeviceName":"/dev/sda1","Ebs":{"VolumeSize":100,"VolumeType":"gp3"}}]' \
#   --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=sequent-step-server}]'

# Example using gcloud (Google Cloud SDK):
# gcloud compute instances create sequent-step-server \
#   --machine-type=n2-standard-8 \
#   --image-family=ubuntu-2204-lts \
#   --image-project=ubuntu-os-cloud \
#   --boot-disk-size=100GB \
#   --boot-disk-type=pd-ssd \
#   --zone=us-central1-a

# Example using Azure CLI:
# az vm create \
#   --resource-group myResourceGroup \
#   --name sequent-step-server \
#   --image Ubuntu2204 \
#   --size Standard_D8s_v3 \
#   --os-disk-size-gb 100 \
#   --storage-sku Premium_LRS \
#   --admin-username azureuser \
#   --generate-ssh-keys

echo "This is a placeholder script. Please edit it with the commands for your cloud provider."
echo ""
echo "Recommended instance: AWS c5.2xlarge (8 vCPUs, 16 GB RAM) or equivalent"
echo "Minimum storage: 50 GB SSD (100 GB recommended)"
