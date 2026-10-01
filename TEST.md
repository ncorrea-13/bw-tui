# Testing against a throwaway Vaultwarden instance

Local, isolated Bitwarden server for testing bw-tui without ever touching (or
displaying) your real vault. Uses Podman + Vaultwarden + a Caddy TLS proxy
(the official `bw` CLI refuses plain HTTP, even for `localhost`).

## Setup

```bash
podman network create vw-test-net

podman run -d --name vw-test --network vw-test-net \
  -v vw-test-data:/data docker.io/vaultwarden/server:latest

podman run -d --name vw-test-proxy --network vw-test-net -p 8443:8443 \
  docker.io/caddy:latest \
  caddy reverse-proxy --from localhost:8443 --to vw-test:80 --internal-certs
```

`--from` needs a hostname (`localhost:8443`), not a bare port (`:8443`) —
Caddy only enables TLS/`--internal-certs` when a host is present.

## Create the test account

`bw register` no longer exists in the official CLI (account creation moved
to the web vault only). Open in a browser and accept the self-signed cert
warning once:

```
https://localhost:8443/#/register
```

Sign up with dummy credentials, e.g. `test@example.com` / `fakepass123`.
Add a few dummy vault items from the web UI before testing bw-tui.

## Point bw-tui's `bw` CLI at it

```bash
bw config server https://localhost:8443
export NODE_TLS_REJECT_UNAUTHORIZED=0   # self-signed cert, test-only, never set this globally
bw login test@example.com fakepass123
```

You may see a `KeyIdBackfillError` / 404 on
`/api/accounts/key-management/user-key-id` — harmless, it's a newer bw CLI
migration endpoint that Vaultwarden doesn't implement yet. Login still
succeeds; confirm with `bw status`.

## Run bw-tui

```bash
BW_SESSION=$(bw unlock --raw)
export BW_SESSION
./bash/bw-tui.sh
```

## Teardown

```bash
podman rm -f vw-test vw-test-proxy
podman volume rm vw-test-data
podman network rm vw-test-net

bw logout
bw config server https://vault.bitwarden.com   # or your real Vaultwarden URL
unset NODE_TLS_REJECT_UNAUTHORIZED BW_SESSION
```
