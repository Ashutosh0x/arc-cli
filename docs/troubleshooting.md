# Troubleshooting ARC CLI

This guide covers the most common issues users encounter when setting up and running ARC.

## Networking & Provider Issues

### `Error: ARC is offline. Could not reach cloud providers.`
**Cause**: The startup network health probe failed to reach the configured API endpoints (e.g. `api.anthropic.com`).
**Solution**:
1. Check your internet connection.
2. If you are behind a corporate proxy, ensure `HTTP_PROXY` and `HTTPS_PROXY` env vars are accurately set. ARC's `reqwest` client will automatically respect them.
3. To bypass the probe, run in forced offline mode or rely solely on local endpoints (Ollama).

### `Error: Rate limit exceeded (429)`
**Cause**: You have exhausted your tier budget on Anthropic/OpenAI or hit a concurrent stream limit.
**Solution**:
ARC includes an automatic exponential backoff mechanism, but if it fundamentally fails:
- Check your provider billing dashboard.
- Downgrade the active model using `arc --model gpt-4o-mini`. 
- Ensure your `TokenBudget` is configured in `arc config edit`.

## Authentication Issues

### `Error: Failed to unlock OS keyring`
**Cause**: ARC stores API keys natively and securely via the `keyring` crate. On Linux, this requires `libsecret` or `dbus`. On headless servers (SSH), the keyring might be locked.
**Solution**:
1. **Linux Headless**: Use `export ARC_API_KEY_...` environment variables instead, which bypasses keyring requirements.
2. Run `arc auth status` to isolate which provider key is causing access issues.

## Git & File Context Issues

### `Warning: Skipping file context extraction > 1MB`
**Cause**: By default, ARC blocks parsing gigantic files into the prompt to save you money on tokens and prevent context overflow.
**Solution**:
- If you genuinely need the file analyzed, update the `.arcignore` to manually define included paths, or bump the threshold using `arc config edit`.

## Database / Memory Issues

### `Database lock conflict`
**Cause**: The `.arc/` directory contains an embedded `redb` database for memory checkpointing. You cannot run two `arc` interactive REPLs targeting the same `.arc/` directory concurrently.
**Solution**:
- Close the other active terminal running `arc` in this repository.
- If no terminal is running, delete the `.arc/arc_history.db.lock` file manually (an unclean exit may have left it behind).
