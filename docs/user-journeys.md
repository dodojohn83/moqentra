# User Journeys

## Tenant Administrator

1. Sign in through OIDC / enterprise identity provider.
2. Create a tenant and configure default resource quotas.
3. Invite users and assign roles (data engineer, annotator, reviewer, algorithm engineer).
4. Review audit logs and cross-tenant access reports.
5. Approve quota exception requests.

## Data Engineer

1. Create a project inside a tenant.
2. Upload images/videos or import from an S3-compatible bucket.
3. Wait for media metadata extraction and checksum validation.
4. Freeze the dataset version and attach a label schema.
5. Export COCO / LabelU format for downstream use.

## Annotator

1. Open the annotation project in the web console.
2. LabelU-Kit loads the task list and media.
3. Draw rectangles, polygons, or trajectories; save drafts.
4. Submit annotations for review.
5. Receive reviewer feedback and resubmit corrections.

## Reviewer

1. Open the review queue for a project.
2. Compare annotations against ground truth or consensus.
3. Approve or reject each task with comments.
4. Export approved annotations and trigger a new dataset version.

## Algorithm Engineer

1. Select a frozen dataset version and a training template.
2. Configure hyperparameters, resource class, and output model name.
3. Submit a training job and monitor metrics/logs in real time.
4. Inspect checkpoints and pick the best epoch.
5. Register the trained model as a new model version.

## DevOps / Platform Operator

1. Choose deployment mode (single-node compose or Kubernetes Helm).
2. Install PostgreSQL, MinIO, control plane, node agent, and worker images.
3. Configure OIDC, object storage, and hardware profiles.
4. Upgrade to a new release using the documented migration path.
5. Restore from backup and verify data integrity.

## Ecosystem Developer

1. Generate an API key scoped to a project.
2. Use the partner SDK to subscribe to training events.
3. Trigger a training job and poll for completion.
4. Download the published model artifact using a presigned URL.

## Cross-Cutting Rules

- Every resource accessed by a journey is scoped to a `TenantId` and `ProjectId`.
- Every mutating journey produces an audit event.
- Every long-running journey (training, conversion, deployment) is resumable and
  idempotent.
