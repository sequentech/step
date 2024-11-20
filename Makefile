.PHONY: up
up:
	nix run .devcontainer/tools#devcontainer -- up --remove-existing-container --workspace-folder .

.PHONY: check-env-forward-ports
check-env-forward-ports:
ifndef HOST
	$(error HOST is undefined. Please, re-run with the environment variable HOST set that will forward ports from that host on your machine, like this: "make HOST=host.example.com forward-ports")
endif

.PHONY: forward-ports
forward-ports: check-env-forward-ports
	ssh -fNT -L 3000:${HOST}:3000 \
	  -L 3002:${HOST}:3002 \
	  -L 3322:${HOST}:3322 \
	  -L 5005:${HOST}:5005 \
	  -L 8000:${HOST}:8000 \
	  -L 8080:${HOST}:8080 \
	  -L 8083:${HOST}:8083 \
	  -L 8090:${HOST}:8090 \
	  -L 8400:${HOST}:8400 \
	  -L 9000:${HOST}:9000 \
	  -L 9002:${HOST}:9002 \
	  hulk.ereslibre.net
