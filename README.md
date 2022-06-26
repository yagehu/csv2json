# csv2json

See `tests/api` for integration tests and API.

## Highlights

### Memory efficient CSV deserialization

The CSV extractor implementation in `csv.rs` takes advantage of the fact that,
unlike JSON, CSV can be processed one line at a time. Since the buffer is
cleared after each processed line, the buffer does not need to grow to
accomodate the entire content size. For CSVs with a large number of lines, this
should reduce memory usage and allow us to service more requests concurrently.

### Generic CSV extractor implementation

The CSV deserialization logic is implemented as an `actix-web`
[extractor](https://actix.rs/docs/extractors/). This allows type-safe code
reuse. Low-level deserialization into `serde` data structures is handled by the
`csv` crate.

## Limitations

1. Although the JSON spec allows the character encoding to be UTF-8, UTF-16, or
   UTF-32, this implementation only accepts UTF-8 encoded CSV input. Thus, the
   only supported `Content-Type` header value is `text/csv; charset=utf-8`.
2. CSV size is limited to a hard-coded value. Making it configurable should be
   trivial.
3. The handlers are monolithic. To scale up the codebase, I would refactor them
   into at least controllers and repositories. All of the logic would go into
   controllers while repositories acts as the persistence layer.
4. Server runs on `localhost:8000` and is not configurable.

## Development

To run migrations, you need to install `sqlx-cli`:

```
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

`up.bash` uses Docker Compose to run a development Postgres container and runs
migrations with `sqlx-cli`. It assumes you have
[Compose V2](https://docs.docker.com/compose/#compose-v2-and-the-new-docker-compose-command)
installed.

```
. ./env.bash
./up.bash
```

To run the server:

```
cargo run
```

You can optionally pipe it into `bunyan` for prettier logs:
`cargo run | bunyan`.

To teardown the development database:

```
docker compose down
```
