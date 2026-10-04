# Como o quadro funciona

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

## A coluna compõe a etapa

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

## Uma raia move o card só até onde você deixa

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

## Um orquestrador, quando você quiser

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
