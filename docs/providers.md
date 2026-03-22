# Multi-Model Provider Hub

The `arc-providers` crate connects ARC to 6 LLM providers through a unified streaming interface. All providers support Server-Sent Events (SSE) streaming and tool-use (function calling).

## Supported Providers

### Anthropic Claude
- **Endpoint:** `https://api.anthropic.com/v1/messages`
- **Models:** `claude-sonnet-4-20250514`, `claude-3-haiku-20240307`
- **Auth:** `ANTHROPIC_API_KEY`
- **Context:** 200K tokens
- **Features:** Native tool-use with XML content blocks, streaming, vision
- Default provider when `ANTHROPIC_API_KEY` is set

### Groq (LPU Inference)
- **Endpoint:** `https://api.groq.com/openai/v1/chat/completions`
- **Models:** `llama-3.3-70b-versatile`, `llama-3.1-8b-instant`, `mixtral-8x7b-32768`
- **Auth:** `GROQ_API_KEY`
- **Context:** 128K tokens
- **Features:** OpenAI-compatible API, tool calling, streaming. Fastest inference speeds via Groq LPU hardware
- Switch to Groq: `/provider groq`

### xAI Grok
- **Endpoint:** `https://api.x.ai/v1/chat/completions`
- **Models:** `grok-4.20-0309-non-reasoning`, `grok-4-1-fast-non-reasoning`, `grok-4.20-multi-agent-0309`
- **Auth:** `XAI_API_KEY`
- **Context:** 2M tokens
- **Features:** OpenAI-compatible API, tool calling (function calling + web search + X search), streaming, 2M context window
- Switch to xAI: `/provider xai`

### OpenAI
- **Endpoint:** `https://api.openai.com/v1/chat/completions`
- **Models:** `gpt-4o`, `gpt-4o-mini`, `o3-mini`
- **Auth:** `OPENAI_API_KEY`
- **Context:** 128K tokens
- **Features:** Tool calling, structured outputs, streaming, vision
- Switch to OpenAI: `/provider openai`

### Google Gemini
- **Endpoint:** `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- **Models:** `gemini-2.5-pro`, `gemini-2.5-flash`
- **Auth:** `GEMINI_API_KEY`
- **Context:** 1M tokens
- **Features:** System instructions, massive context budget, vision

### Ollama (Local/Offline)
- **Endpoint:** `http://localhost:11434/api/chat`
- **Models:** Any model installed locally (`llama3.1`, `codellama`, `deepseek-coder`)
- **Auth:** None (local)
- **Context:** Model-dependent
- **Features:** Fully offline, uncensored, no API costs

## Provider Selection

ARC auto-detects the active provider from environment variables in priority order:

```
ANTHROPIC_API_KEY  ->  Anthropic Claude (default)
GROQ_API_KEY       ->  Groq (Llama 3.3)
XAI_API_KEY        ->  xAI Grok
OPENAI_API_KEY     ->  OpenAI (GPT-4o)
```

Switch providers live during a session:
```
/provider groq           # switch to Groq
/provider xai            # switch to xAI Grok
/model llama-3.1-8b-instant  # change model within current provider
/status                  # show current provider + model
```

## Architecture

Groq, xAI, and OpenAI share a unified `OpenAICompatProvider` (`openai_compat.rs`) since all three use the OpenAI chat completions API format. This single implementation handles:
- SSE streaming with `data:` deltas
- Tool calling via `tools` parameter and `tool_calls` response format
- Tool result messages with `role: "tool"` and `tool_call_id`

Anthropic has its own dedicated provider (`anthropic.rs`) due to its different message format (content blocks vs. string content).

## Networking

- HTTP/2 connection pooling (`http2_prior_knowledge`)
- Persistent `reqwest::Client` shared across providers
- Bearer token auth for OpenAI-compatible APIs
- Query parameter auth for Gemini API