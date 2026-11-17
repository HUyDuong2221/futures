# BTC Futures AI Lab

## Deploy with Docker Compose

Configure `.env` from `.env.example`, then build and start the services:

```sh
docker compose up -d --build
```

Open `http://<server-ip>:5173`. The frontend sends `/api` requests to the same
origin, and its Nginx server forwards them to the backend container. This avoids
using the visitor's `localhost` as the API host.

After changing frontend or Docker configuration, rebuild the images with
`docker compose up -d --build`.
