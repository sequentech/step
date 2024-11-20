.PHONY: up
up:
	nix run .devcontainer/tools#devcontainer -- up --remove-existing-container --workspace-folder .
