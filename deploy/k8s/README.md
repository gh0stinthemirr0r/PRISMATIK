# Kubernetes stubs — `prismatik-api`

**Status:** floor scaffold only. Not a production deploy.

Stub manifests (no Secrets, no real image digest):

| File | Kind |
|---|---|
| `namespace.yaml` | `Namespace` `prismatik` |
| `deployment.yaml` | `Deployment` `prismatik-api` |
| `service.yaml` | `Service` (ClusterIP → container port 8080) |

Placeholder image: `ghcr.io/prismatik/prismatik-api:placeholder`.

## Local dry-run (optional)

```bash
kubectl apply --dry-run=client -f namespace.yaml -f deployment.yaml -f service.yaml
```

Do **not** apply these stubs to a shared/prod cluster as-is.

## Author-ops for prod (required before live)

1. **Image** — Build/push a signed `prismatik-api` image; pin by digest (not `:placeholder` / floating tags).
2. **Registry auth** — Wire `imagePullSecrets` (or workload identity) for the target registry.
3. **Secrets / config** — Externalize DB, OIDC, Vault, and provider keys via sealed-secrets / ExternalSecrets / cloud secret manager — never commit Secret YAML here.
4. **TLS / ingress** — Add Ingress (or Gateway) + cert manager; terminate TLS at the edge.
5. **Resources / HPA** — Size requests/limits from load tests; add HPA/PDB as needed.
6. **Observability** — Scrapes, SLO alerts, and runbooks (see Wave 6 `P8-OD-03`).
7. **RBAC / network** — Namespace NetworkPolicies + least-privilege ServiceAccount.
8. **Change control** — Review, apply via CD (not laptop `kubectl apply`), and record the release gate.

Terraform / GitOps overlays remain deferred (`P8-OD-01..02` residual).
