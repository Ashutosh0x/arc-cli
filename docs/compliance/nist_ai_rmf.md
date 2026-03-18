# NIST AI Risk Management Framework (RMF)

ARC CLI’s architecture intrinsically supports the National Institute of Standards and Technology (NIST) AI RMF 1.0.

## 1. Govern
**Cultivate and implement a culture of risk management.**
- Total observability is built into `arc-core` metrics.
- `cargo vet` and `cargo audit` workflows systematically identify upstream vulnerabilities.
- Secret-locking via automated Keychain interactions guarantees secure developer hygiene.

## 2. Map
**Recognize, assess, and prioritize risks.**
- Utilizing `TelemetryStore`, ARC tracks quantitative risk metrics natively. Total cost ($USD) is accurately aggregated to prevent runaway billing risks.
- `Lethal Trifecta` evaluation dynamically assesses the system context during execution (Access to Data + Untrusted Content + Exfiltration Vector) to generate explicit human-in-the-loop alerts.

## 3. Measure
**Employ qualitative and quantitative tools to analyze risk.**
- OpenTelemetry (`opentelemetry_sdk`, OTLP exporter) traces LLM span duration and accuracy.
- `hdrhistogram` tracks Provider latencies down to p99 levels, allowing dynamic SLA validation across Anthropic, Ollama, and Google Gemini backends.

## 4. Manage
**Allocate resources to handle risks.**
- Configurable Sandboxing (`arc-sandbox`) and `PromptGuard` explicitly isolate potentially harmful operations from lateral machine impact.
- Memory thresholds dynamically prune outdated or harmful state logic from the active LLM context.
