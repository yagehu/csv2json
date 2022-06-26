#!/usr/bin/env bash

set -euxo pipefail

docker compose up -d

until \
  docker compose run \
    --env PGPASSWORD="${C2J_DEV_POSTGRES_PASSWORD}" \
    --rm \
    postgres \
    psql -h host.docker.internal \
    --username "${C2J_DEV_POSTGRES_USER}" \
    --port "${C2J_DEV_POSTGRES_PORT}" \
    --dbname "${C2J_DEV_POSTGRES_DATABASE}" \
    --command \\q;
do
  sleep 1;
done

sqlx migrate run
