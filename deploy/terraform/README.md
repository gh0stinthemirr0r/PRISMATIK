# PRISMATIK Terraform stubs (author-ops)

Placeholder modules for future cloud infra. **Not production-ready.**

## Layout

| Path | Purpose |
|---|---|
| `modules/api/` | API service skeleton (image, replicas, env) |
| `environments/dev/` | Dev root that wires the api module |

## Author-ops before prod

- Backend state (S3/GCS + lock) — never local-only for shared envs
- OIDC / workload identity for CI apply
- Secret injection via Vault / Secret Manager — no plaintext in tfvars
- Align digests with cosign/SLSA (P0-SS-06)
- Network policies / private endpoints

## Usage (dev sketch)

```bash
cd deploy/terraform/environments/dev
terraform init
terraform plan
```
