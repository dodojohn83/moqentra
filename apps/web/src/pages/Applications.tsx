import { useState } from "react";
import { useApi } from "../core/useApi";
import { useTenant } from "../core/TenantContext";

/**
 * Application compile surface (R1-WEB-011 simplified).
 * Full React Flow canvas can replace this form; compile uses OpenAPI client.
 */
export function Applications() {
  const api = useApi();
  const { scope } = useTenant();
  const [specJson, setSpecJson] = useState(
    JSON.stringify(
      {
        name: "rtsp-detect-rtmp",
        nodes: [],
        edges: [],
      },
      null,
      2,
    ),
  );
  const [result, setResult] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  async function compile() {
    setError(null);
    try {
      const spec = JSON.parse(specJson);
      // compile endpoint via raw fetch if not generated
      const base = import.meta.env.VITE_API_BASE_URL || "";
      const res = await fetch(`${base}/v1/applications/compile`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
          "X-Tenant-Id": scope.tenantId,
        },
        body: JSON.stringify({ spec }),
      });
      if (!res.ok) {
        const body = await res.text();
        throw new Error(body || `HTTP ${res.status}`);
      }
      const body = await res.json();
      setResult(JSON.stringify(body, null, 2));
      void api;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Compile failed");
    }
  }

  return (
    <section className="page">
      <h1>Applications</h1>
      <p>Edit ApplicationSpec JSON (React Flow editor can mount here).</p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <textarea
        aria-label="ApplicationSpec JSON"
        rows={16}
        style={{ width: "100%", fontFamily: "monospace" }}
        value={specJson}
        onChange={(e) => setSpecJson(e.target.value)}
      />
      <button type="button" onClick={() => void compile()}>
        Compile
      </button>
      {result && (
        <pre aria-label="Compile result">{result}</pre>
      )}
    </section>
  );
}
