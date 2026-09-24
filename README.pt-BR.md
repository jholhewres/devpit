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
- **Agentes são arquivos** — markdown com frontmatter em `~/.devpit/agents/`, e
  os que a sua ferramenta já instalou. Nenhum vem com o devpit: os que você vê
  são os da sua máquina. Nada para recompilar, nada para registrar.
- **Um pane de navegador**, para a página que o projeto está servindo. Abre
  `localhost` em http, que é o que um servidor de dev responde, mantém a
  sessão de cada pane separada, e traz uma sessão já logada do Chrome, do
  Firefox ou do Safari — por navegador e por domínio, nunca sozinho. Um agente
  dirige a página que você der a ele, e nunca lê o que você digita num campo
  de senha.
- **Árvore de arquivos e diffs** ao lado do terminal, para revisar o que o
  agente fez sem precisar sair.
- **Na sua máquina.** Projetos, quadros, conversas e histórico de terminal
  ficam em `~/.devpit`, neste computador. A conta é opcional e, hoje, só
  identifica você — sincronizar entre máquinas não está feito.

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

**Esse é um quadro que alguém montou, não o que você recebe.** Um projeto novo
nasce com essas seis colunas e nada atrás delas: cada raia não executa nada até
você dar uma etapa a ela, o que é um menu na própria raia.

Uma coluna ou não faz nada, ou executa um de três tipos de etapa:

| | `agent` | `session` | `command` |
|---|---|---|---|
| Toma o terminal? | não | **sim** — ele é o terminal alvo | não |
| Serve para | refinar, revisar, verificar | implementar | testes, builds, deploys, lint |
| Devolve | JSON validado por schema, com custo e duração | uma sessão que você conduz | saída em streaming e um código de saída |
| Quantos por vez | vários | vários &mdash; um deles anexado | vários |

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

### Uma raia move o card só até onde você deixa

Uma etapa que termina anota o que aconteceu — saída, código de saída, custo
real. O que vem depois é a configuração da raia, e você escolhe por raia:

| | |
|---|---|
| `manual` | nada se move. O card fica onde está até você arrastar. |
| `ask` | o devpit pergunta, dizendo para qual raia levaria o card. |
| `auto` | ele move, e a etapa da próxima raia roda. |

`manual` é o padrão, e uma raia em `auto` diz isso no quadro. Não há
orquestrador atrás disso: nada agenda trabalho, nada tenta de novo, e uma
sequência de raias em `auto` é uma que você montou raia por raia e consegue
ver.

### Um orquestrador, quando você quiser

Um orquestrador é um chat que você conduz e que enxerga todos os projetos de uma
vez. Criado no topo do rail — com qual conta do Claude Code ele roda e um nome —
ele lista os projetos e seus quadros, vê as sessões da conta rodando agora e
pode entregar o trabalho de um card a uma sessão nova: no checkout do próprio
card, ligada ao card, no quadro como qualquer outra. Ele ouve essas sessões entre
as suas mensagens e diz no chat o que elas relataram.

Quem decide continua sendo você. Ele nunca move um card para uma raia com etapa,
nada do que ele faz roda num temporizador, e uma mensagem dele não aprova nada em
outra sessão — quando uma espera por você, você responde pelo painel Sessions do
orquestrador, como você mesmo.

**Sem card, só o terminal.** Abra o devpit, digite, e sua CLI de agente se
comporta exatamente como sempre se comportou. O quadro é uma fonte opcional de
trabalho, não uma cancela.

## Requisitos

- **Linux**, X11 ou Wayland, é o build. macOS tem um build universal sem
  assinatura nem notarização; Windows ainda não é construído.
- **`tmux`**. Os terminais são painéis do tmux, e é por isso que eles
  sobrevivem à janela.
- **[Claude Code](https://claude.com/claude-code)** (`claude` no PATH). Chat,
  etapas de agente e de sessão passam por ele; é a única CLI que o devpit sabe
  conduzir hoje.

## Instalando

Linux e macOS estão na [última release][releases]. No Linux, prefira o
AppImage, a não ser que você tenha motivo para não: é o que se atualiza
sozinho.

Os comandos abaixo dizem `0.1.4` porque a versão faz parte do nome do arquivo.
Confira na página de releases qual é a atual — ou deixe um devpit instalado se
atualizar sozinho e nunca mais digite uma versão.

**AppImage (Linux).** O devpit procura uma versão nova, confere a assinatura,
instala por cima de si mesmo e reabre. Seus terminais continuam rodando no meio
disso — são sessões do tmux, e o tmux não cai junto com a janela.

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/devpit_0.1.4_amd64.AppImage
chmod +x devpit_0.1.4_amd64.AppImage
./devpit_0.1.4_amd64.AppImage
```

**`.deb` (Debian, Ubuntu).** O devpit baixa a versão nova e confere, e então
mostra o comando sem nunca executá-lo. Instalar um pacote do sistema é pedir
root, e o devpit não pede root no seu lugar.

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/devpit_0.1.4_amd64.deb
sudo apt install ./devpit_0.1.4_amd64.deb
```

**`.dmg` (macOS).** Só Apple silicon — M1 ou mais novo. Ainda não há build
para Intel: o runner que o produz é a última imagem Intel do GitHub e está
saindo de circulação, então a resposta para um Mac Intel é um binário
universal, que ainda não é construído. O devpit se atualiza aqui: baixa o
`.app`, confere a assinatura e se troca.

O download **não é assinado com um Developer ID da Apple nem notarizado**, então
a primeira abertura é recusada pelo Gatekeeper com "devpit está danificado" ou
"não pode ser aberto". Isso é o certificado que falta falando, não o arquivo.
Abra uma vez com botão direito → Abrir, ou tire a quarentena você mesmo:

```sh
xattr -dr com.apple.quarantine /Applications/devpit.app
```

Se for fazer isso, confira antes o `SHA256SUMS` — logo abaixo.

**Windows** ainda não é compilado. O terminal é tmux e tmux não existe lá, então
é um port e não um build; não está nesta release.

Toda release traz um `SHA256SUMS`, e conferir é uma linha:

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
```

O `.sig` ao lado de cada pacote é do updater, e não substitui isso: ele é o que
um devpit já instalado confere antes de se trocar, contra uma chave pública
compilada no binário que você já está rodando. Um primeiro download não tem
esse binário para conferir com ele — é para isso que serve o `SHA256SUMS`.

A checagem automática é um botão em Configurações → General, e vem ligada até
você desligar.

[releases]: https://github.com/jholhewres/devpit/releases/latest

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

## Mais

- [The model](docs/the-model.md) — os sete objetos de que tudo é feito
- [Architecture](docs/architecture.md) — estrutura, e as regras que o build cobra

## Licença

[Apache 2.0](LICENSE).
