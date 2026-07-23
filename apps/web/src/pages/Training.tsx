import { FormEvent, useCallback, useEffect, useState } from "react";
import { useApi } from "../core/useApi";
import { useTenant } from "../core/TenantContext";

interface JobRow {
  id?: string;
  experimentId?: string;
  experiment_id?: string;
  state?: string;
}

export function Training() {
  const api = useApi();
  const { scope } = useTenant();
  const [jobs, setJobs] = useState<JobRow[]>([]);
  const [experimentId, setExperimentId] = useState("");
  const [datasetVersionId, setDatasetVersionId] = useState("");
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!scope.tenantId) return;
    try {
      const page = await api.listTrainingJobs({ xTenantId: scope.tenantId });
      setJobs((page.items ?? []) as JobRow[]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load jobs");
    }
  }, [api, scope.tenantId]);

  useEffect(() => {
    void load();
  }, [load, scope.projectId]);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (!scope.projectId || !scope.tenantId) {
      setError("Select tenant and project first");
      return;
    }
    setError(null);
    try {
      await api.createTrainingJob({
        xTenantId: scope.tenantId,
        createTrainingJobRequest: {
          project_id: scope.projectId,
          experiment_id: experimentId,
          dataset_version_id: datasetVersionId,
          code_digest:
            "sha256:a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333",
          image_digest:
            "sha256:eef93e1d14482804277fca0172464032d1a4fdbcc338524059fa1e861454ad4d",
          argv: ["train"],
        },
      });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Submit failed");
    }
  }

  async function cancel(id: string) {
    if (!scope.tenantId) return;
    try {
      await api.cancelTrainingJob({ xTenantId: scope.tenantId, id });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Cancel failed");
    }
  }

  return (
    <section className="page">
      <h1>Training jobs</h1>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <form onSubmit={onSubmit}>
        <label>
          Experiment ID
          <input
            value={experimentId}
            onChange={(e) => setExperimentId(e.target.value)}
            required
          />
        </label>
        <label>
          Dataset version ID
          <input
            value={datasetVersionId}
            onChange={(e) => setDatasetVersionId(e.target.value)}
            required
          />
        </label>
        <button type="submit">Submit job</button>
      </form>
      <ul>
        {jobs.map((j) => (
          <li key={j.id}>
            <code>{j.id}</code> exp={j.experimentId ?? j.experiment_id} state=
            {j.state}{" "}
            {j.id && (
              <button type="button" onClick={() => void cancel(j.id!)}>
                Cancel
              </button>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
