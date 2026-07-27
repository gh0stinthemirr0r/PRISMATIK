variable "name" {
  type        = string
  description = "Service name"
  default     = "prismatik-api"
}

variable "image" {
  type        = string
  description = "Container image (digest-pinned in prod)"
  default     = "ghcr.io/prismatik/prismatik-api:placeholder"
}

variable "replicas" {
  type        = number
  description = "Desired replica count"
  default     = 1
}

output "service_name" {
  value = var.name
}

output "image" {
  value = var.image
}

output "replicas" {
  value = var.replicas
}

# Floor: no cloud provider resources — wire aws/google/azurerm in author-ops.
