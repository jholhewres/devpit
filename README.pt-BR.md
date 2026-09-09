<p align="center">
  <img src="assets/images/devpit-mark.png" alt="" width="128">
</p>

<h1 align="center">devpit</h1>

<p align="center">
  Um espaço de trabalho desktop para tocar agentes de código.
</p>

<p align="center">
  <a href="https://github.com/jholhewres/devpit/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jholhewres/devpit/ci.yml?branch=main&style=flat-square&label=ci" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/licen%C3%A7a-Apache%202.0-blue?style=flat-square" alt="Licença: Apache 2.0"></a>
  <img src="https://img.shields.io/badge/rust-1.93+-dea584?style=flat-square&logo=rust&logoColor=white" alt="Rust 1.93+">
  <img src="https://img.shields.io/badge/status-inicial-e2795b?style=flat-square" alt="Status: inicial">
</p>

<p align="center">
  <a href="README.md">English</a> &middot; <b>Português</b>
</p>

---

## Para que serve

Agente é fácil de começar e difícil de acompanhar. Cinco terminais depois, você
já não sabe o que está rodando, quanto custou, nem qual deles você estava lendo.

O devpit existe para manter **cada etapa do trabalho visível e sob a sua mão** —
um terminal que você de fato está olhando, um quadro que diz onde cada frente
está, e um número em cada chamada de agente.

## O que ele faz

- **Um terminal alvo por projeto.** Trocar de frente troca o que está atrelado a
  ele. A sessão anterior continua rodando em segundo plano; só para de ocupar a
  tela.
- **Sessões que sobrevivem à janela.** Feche o devpit, abra de novo, e o que
  estava rodando continua rodando.
- **Uma worktree do git por frente de trabalho**, criada e removida sem você
  pedir.
- **Um quadro por projeto.** Mover um card executa trabalho de verdade — no
  terminal que você já tem aberto, não em um novo.
- **As colunas são suas.** Renomeie, reordene, crie as suas, decida qual executa
  o quê.
- **Teto de gasto em toda chamada de agente**, e o custo real escrito no card
  quando ela termina.
- **Agentes são arquivos** — markdown com frontmatter em `~/.devpit/agents/`.
  Escreva os seus ao lado dos que já vêm. Nada para recompilar, nada para
  registrar.
- **Árvore de arquivos e diffs** ao lado do terminal, para revisar o que o
  agente fez sem precisar sair.
- **Local-first.** Uma máquina, sem conta, sem nuvem.

## Como o quadro funciona

```
     [inbox]     [refinar]    [revisar]     [fazendo]   [conferir]   [entregar]
        │            │            │             │            │            │
   você escreve    agente       agente       uma sessão    agente     comando seu
     o card       headless     headless      que você     headless    (testes,
                                              conduz                    deploy)
                      └────────────┘                          └────────────┘
                       sem terminal                            sem terminal
                                                 ▲
                                      o único terminal alvo
```

Uma coluna ou não faz nada, ou executa um de três tipos de etapa:

| | `agent` | `session` | `command` |
|---|---|---|---|
| Toma o terminal? | não | **sim** — ele é o terminal alvo | não |
| Serve para | refinar, revisar, verificar | implementar | testes, builds, deploys, lint |
| Devolve | JSON validado por schema, com custo e duração | uma sessão que você conduz | saída em streaming e um código de saída |
| Quantos por vez | vários | **um** | vários |

O fluxo acima é só o quadro padrão de um projeto novo.

**Sem card, só o terminal.** Abra o devpit, digite, e sua CLI de agente se
comporta exatamente como sempre se comportou. O quadro é uma fonte opcional de
trabalho, não uma cancela.

## Requisitos

- Rust 1.93+, Node 22+, pnpm
- `tmux`
- a CLI de agente que você usa
- no Linux, os pacotes de desenvolvimento do WebKitGTK:

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

## Mais

- [The model](docs/the-model.md) — os sete objetos de que tudo é feito
- [Architecture](docs/architecture.md) — estrutura, e as regras que o build cobra

## Licença

[Apache 2.0](LICENSE).
