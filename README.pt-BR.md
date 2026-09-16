<p align="center">
  <img src="web/src/assets/brand/mark.png" alt="" width="128">
</p>

<h1 align="center">devpit</h1>

<p align="center">
  Um app nativo para controlar seus agentes de código.
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
- **Uma worktree do git por frente de trabalho**, criada quando uma etapa
  precisa de checkout e removida quando você mandar — nunca com trabalho não
  commitado dentro.
- **Um quadro por projeto.** Mover um card executa trabalho de verdade — no
  terminal que você já tem aberto, não em um novo.
- **As colunas são suas.** Renomeie, reordene, crie as suas, decida qual executa
  o quê.
- **Cada coluna compõe a própria etapa** — qual agente, em qual modelo, com
  que fatia do contexto do card.
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
      [inbox]       [refinar]      [revisar]      [fazendo]     [conferir]     [entregar]
         │              │              │              │              │              │
   você escreve      planner        critic        executor       verifier        comando
      o card         · opus         · opus        · sonnet       · sonnet          seu
         —            agent          agent         session         agent         command
                        └──────────────┘                             └──────────────┘
                         sem terminal                                 sem terminal
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

### A coluna compõe a etapa

Uma etapa não é "chama um agente". É uma receita que a coluna guarda, e mover
o card a resolve numa invocação:

| | |
|---|---|
| `agent` | **quem** faz — `architect`, `executor`, `reviewer` |
| `model` | em **qual modelo** aquela chamada roda |
| `inject` | que parte do contexto do card chega nela |
| `capUsd` | o teto que ela pode gastar |
| `expects` | o formato que a resposta tem que cumprir |

Os agentes vêm da sua máquina — os que você escreveu em `~/.devpit/agents/` e os
que sua ferramenta já instalou. O devpit **referencia, nunca copia**: quem é
dono de um catálogo continua sendo.

O ponto é escopo. Vinte agentes disponíveis em todo lugar acabam carregados em
todo lugar, e você paga por tudo isso em cada chamada. A coluna diz que *esta*
etapa é aquele agente, naquele modelo, com aquela fatia do card — e é só isso
que é enviado.

Skills não entram nessa lista. A CLI oferece todas ou nenhuma a um turno, então
a etapa pede uma no prompt — `/tdd` — do mesmo jeito que você pediria numa
sessão, e uma etapa que tenta declará-las é recusada ao ser salva.

### Nada avança sozinho

Uma etapa que termina não empurra o card. Ela anota o que aconteceu — saída,
código de saída, custo real — e para. O próximo movimento é seu.

Não existe orquestrador no devpit porque o orquestrador é você. Essa ausência é
o produto, não uma lacuna nele: orquestração que se mantém sozinha é
orquestração que você não vê, e perder de vista o trabalho é o problema que isto
ataca.

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
