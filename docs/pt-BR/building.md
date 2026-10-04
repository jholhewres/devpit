# Compilando e rodando o devpit

## Compilando

- Rust 1.93+, Node 22+, pnpm
- os pacotes de desenvolvimento do WebKitGTK:

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
                 build-essential curl file libssl-dev libayatana-appindicator3-dev
```

## Rodando

```sh
make setup    # dependências
make dev      # o app, com hot reload
make test     # guardas, testes Rust, testes do front
make build    # bundle de release, front embutido
```

`make` sozinho lista todos os alvos.
