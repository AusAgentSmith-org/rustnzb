---
name: loki
description: Query rustnzbd logs from the centralized Loki instance on the VPS
disable-model-invocation: true
allowed-tools: Bash(curl *)
user-invocable: true
argument-hint: "[filter] [--since duration] [--limit N]"
---

# Query rustnzbd Logs from Loki

Query rustnzbd logs from the centralized Loki stack. Logs are shipped via a Promtail sidecar on Node B.

## Usage

- `/loki` — Recent rustnzbd logs (last 10 minutes)
- `/loki ERROR` — Error lines only
- `/loki "download complete"` — Filter for specific text
- `/loki --since 1h` — Last hour
- `/loki --since 30m --limit 100` — Last 30 min, up to 100 lines

## Loki endpoint

Direct via Tailscale: `http://100.96.114.15:3100`

## Steps

1. Parse `$ARGUMENTS`:
   - Text words → filter (`|=` or `|~` for multiple terms)
   - `--since <duration>` → time range (default `10m`)
   - `--limit <N>` → max lines (default `50`)

2. Build LogQL query — always scoped to rustnzbd:
   ```logql
   {container="rustnzbd", host="NodeB"}
   ```
   Add `|= "<filter>"` if filter text provided.

3. Execute:
   ```bash
   curl -s -G 'http://100.96.114.15:3100/loki/api/v1/query_range' \
     --data-urlencode 'query={container="rustnzbd", host="NodeB"} |= "<filter>"' \
     --data-urlencode 'limit=<N>' \
     --data-urlencode 'since=<duration>'
   ```

4. Format output:
   ```bash
   | python3 -c "
   import json, sys
   data = json.load(sys.stdin)
   results = data.get('data', {}).get('result', [])
   lines = []
   for stream in results:
       for ts, line in stream.get('values', []):
           lines.append((int(ts), line[:300]))
   lines.sort()
   for ts, line in lines:
       print(line)
   if not lines:
       print('No results - is promtail running? Try: /deploy --logging')
   "
   ```

5. If no results, suggest:
   - Check promtail is running: `/logs` and look for promtail
   - Broaden time range with `--since`
   - Verify in Grafana: http://46.250.255.234:3000

## LogQL examples

```logql
# All rustnzbd logs
{container="rustnzbd", host="NodeB"}

# Errors only
{container="rustnzbd", host="NodeB"} |~ "ERROR|error|Error"

# Download activity
{container="rustnzbd", host="NodeB"} |~ "download|Download"

# Connection issues
{container="rustnzbd", host="NodeB"} |~ "connection|timeout|refused"
```
