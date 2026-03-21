---
name: deploy
description: Deploy rustnzbd on Node B (pull latest image, restart container)
disable-model-invocation: true
allowed-tools: Bash(ssh *), Bash(curl *)
user-invocable: true
argument-hint: "[--build] [--down] [--logging]"
---

# Deploy rustnzbd

Manage the rustnzbd Docker deployment on Node B (192.168.0.30).

## Usage

- `/deploy` — Pull latest image and restart
- `/deploy --build` — Build from source and restart
- `/deploy --down` — Stop the stack
- `/deploy --logging` — Deploy with Promtail logging to Loki enabled

## Steps

1. SSH to Node B: `ssh -o ConnectTimeout=10 sprooty@192.168.0.30`
2. Working directory: `cd ~/rustnzbd`

3. If `--down`:
   ```bash
   docker compose down
   ```

4. If `--logging`:
   ```bash
   LOKI_URL=http://100.96.114.15:3100 HOSTNAME=NodeB COMPOSE_PROFILES=logging docker compose pull
   LOKI_URL=http://100.96.114.15:3100 HOSTNAME=NodeB COMPOSE_PROFILES=logging docker compose up -d
   ```

5. If `--build` (build from source — requires repo on Node B):
   ```bash
   docker compose up -d --build
   ```

6. Default (pull latest published image):
   ```bash
   docker compose pull
   docker compose up -d
   ```

7. Post-deploy checks:
   - Wait 5s
   - Health check: `curl -sf http://localhost:9095/api/status`
   - Container status: `docker compose ps`
   - If logging enabled: `docker logs rustnzbd-promtail-1 --tail 5`

## CI/CD

rustnzbd has a GitHub Actions workflow (`.github/workflows/docker-deploy.yml`) that:
1. Builds and pushes to GHCR + Docker Hub on push to `main`
2. Auto-deploys on Node B (self-hosted runner)

So `/deploy` is mainly for manual re-deploys or config changes.

## Ports

| Port | Service |
|------|---------|
| 9095 (host) → 9090 (container) | rustnzbd web UI + API |
