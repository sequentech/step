<!--
SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->

# Welcome to Sequent air-gapped environment

## Instructions

First decompress the folder

### Windows (x86-64)

#### 1. Enable WSL Feature

On the machine Open `Windows PowerShell` as Administrator
(`Run as Administrator`) and execute the following to  enable WSL 2:

```bash
dism /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart
dism /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart
```

#### 2. Install Desktop Desktop

Install Docker Desktop clicking the file `docker_desktop_installer.exe` inside the
`docker-desktop` folder.

Then restart the computer.

#### 3. Install the WSL Kernel Update

Install Docker Desktop clicking the file `wsl_update_x64.msi` inside the
`docker-desktop` folder.

#### 4. Install the Ubuntu WSL Distro

Now, you need to install the Ubuntu Linux distribution we will be using to run
the system.

Navigate to the folder `docker-desktop` and run the following command to install
the distribution:

```bash
# install ubuntu
Add-AppxPackage ubuntu.appx
```

### 5. Configure WSL defaults

Now let's configure WSL 2 as the default:

```bash
wsl --set-version Ubuntu 2
```

Then you need to set ubuntu as the default WSL linux distro:

```bash
# set ubuntu as the default distro
wsl --setdefault ubuntu
```

And check that this distro is actually the default by executing the following
command:


```bash
# list distros and ensure the default is ubuntu
wsl -l
```

The output should be something like:

```
Windows Subsystem for Linux Distributions:
Ubuntu (default)
docker-desktop
```

#### 6. Configure WSL in Docker Desktop

In Docker Desktop, activate the WSL integration in Docker Desktop Settings
(more info in https://docs.docker.com/desktop/wsl/):
- Enter into `Docker Desktop`
- Navigate to `Settings` (the gear button on the top blue header)
- Click `General` in the sidebar and ensure `Use WSL 2 based engine` is enabled
- Click in `Resources` in the sidebar and then click `WSL integration`
- Ensure `Enable integration with my default WSL distro` is enabled
- Ensure `ubuntu` is enabled under the
 `Enable integration with additional distros` section
- Click `Apply & restart` button if you had to apply any changes

#### 7. Executing Sequent Step Platform

In order to execute Sequent Step, you have to run the following command:

```bash
$ wsl sudo bash ./up
```

Once that it has been imported and started, you can visit the different services
at their endpoints:

- Admin portal: http://localhost:3002

### Linux/Mac (x86-64)

#### 1. Install Docker Desktop

- Docker Desktop installed

#### 2. Export Online Platform Trustees

In the online platform, once logged in as an admin user, go to Settings > Trustees > Export.
This will download an encrypted zip file and it will open a modal with the password to the zip.
Copy the encrypted file to the Ubuntu Desktop computer.

#### 3. Executing Sequent Step Platform

In order to execute Sequent Step Platform, you have to run the following
command as a root user:

```bash
sudo su -
$ ./up <trustees_ezip> <password> <excel_path>
```

Replace `<trustees_ezip>` with the path to the encrypted zip with the trustees
data and `<password>` with the password to the ezip. The <excel_path> should be
the path to the excel file, for example janitor/import-data/10-11-2024-field-test-preparations.xlsx

Once that it has been imported and started, you can visit the different services
at their endpoints:

- Admin portal: http://localhost:3002