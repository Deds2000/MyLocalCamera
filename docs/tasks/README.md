# Cartões de tarefa — como funcionam

Um **cartão** é uma tarefa pequena e **autossuficiente**, pensada para caber com folga no contexto de um modelo de 200k tokens (a meta é ~10k tokens por tarefa, incluindo leitura, código e saídas de comandos).

## Regras de tamanho
- Um cartão = **uma** entrega verificável (≈ 1–3 horas de trabalho humano).
- Um arquivo de código novo ≤ ~300 linhas. Mais que isso? Dividir em módulos.
- O cartão **embute** o trecho da especificação que importa (não mande o agente ler `protocol.md` inteiro).
- Todo cartão termina com **comando de verificação** e **critério de pronto** objetivos.

## Estrutura de um cartão (ver `_template.md`)
`Cabeçalho` (fase, executor, depende de, contexto estimado) → `Objetivo` → `Leia só isto` → `Contexto embutido` → `Arquivos` → `Passos` → `Esqueleto de código` → `Testes a escrever` → `Verificação` → `Não faça` → `Critério de pronto` → `Commit`.

## Executor
| Valor | Significado |
|---|---|
| `agente` | O modelo executa sozinho no repositório |
| `humano` | Exige aparelho, hardware, instalação com UAC ou julgamento visual. O agente **para** e entrega o roteiro ao usuário |
| `agente+humano` | O agente prepara (código/script); o humano executa a parte manual e devolve o resultado (o cartão diz o formato) |

## Convenções de ID
`F<fase>-T<nn>` (ex.: `F1-T02a`). Spikes: `F0-S1`, `F0-S2`, `F0-S3`, `F0-S5`, `F8-S4`. Portão da fase: `F<fase>-GATE`. O arquivo se chama `<ID>.md` dentro de `docs/tasks/fase-<N>/`.

## O que fazer quando faltar um cartão
Fases 2–9 ainda não têm cartões prontos porque dependem dos vereditos dos spikes S1, S2, S3 e S5 (e do S4, na Fase 8). Quando o `PROGRESS.md` apontar para uma tarefa marcada **(gerar)**: **pare** e peça ao usuário para gerar os cartões da fase (ele fará isso com o autor do plano). Não improvise a tarefa a partir do arquivo da fase.

## Relatórios
Cada fase fecha com `F<N>-GATE`: roda todos os testes, preenche `docs/tests/fase-N.md` (template em `docs/tests/_template.md`) e só então marca a fase como concluída.
