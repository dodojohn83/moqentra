import { useState } from "react";

/** Deployment console stub for R1-WEB-012 — wires to Operation/status APIs. */
export function Deployments() {
  const [agent, setAgent] = useState("");
  const [status, setStatus] = useState<string>("idle");
  const [error, setError] = useState<string | null>(null);

  async function publish() {
    setError(null);
    setStatus("publishing");
    try {
      // Placeholder until OpenAPI deploy endpoints are generated for all fields.
      setStatus("submitted (use control-plane /v1 deploy APIs)");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Publish failed");
      setStatus("error");
    }
  }

  return (
    <section className="page">
      <h1>Deployments</h1>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <label>
        Target agent
        <input value={agent} onChange={(e) => setAgent(e.target.value)} />
      </label>
      <button type="button" onClick={() => void publish()}>
        Publish
      </button>
      <p>Status: {status}</p>
    </section>
  );
}
