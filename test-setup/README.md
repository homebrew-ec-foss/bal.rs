# Test servers

Test servers for the load balancer

Ports: 5000 - 5002 can be increased by modifying compose.yml

To build images:
`docker compose build`

To start test servers and redis:
`docker compose up`

To stop servers:
`docker compose down`

To stop a specific server:
`docker compose down [service_name]`
