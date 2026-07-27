module "api" {
  source   = "../../modules/api"
  name     = "prismatik-api"
  image    = "ghcr.io/prismatik/prismatik-api:placeholder"
  replicas = 1
}

output "api_service" {
  value = module.api.service_name
}
