# Cartões de tarefa — como funcionam

Um **cartão** é uma tarefa pequena e **autossuficiente**, pensada para caber com folga no contexto de um modelo de 200k tokens (a meta é ~10k tokens por tarefa, incluindo leitura, código e saídas de comandos).

## Regras de tamanho
- Um cartão = **uma** entrega verificável (≈ 1–3 horas de trabalho do agente).
- Um arquivo de código novo ≤ ~300 linhas. Mais que isso? Dividir em módulos.
- O cartão **embute** o trecho da especificação que importa (não mande o agente ler `protocol.md` inteiro).
- Todo cartão termina com **comando de verificação** e **critério de pronto** objetivos.

## Estrutura de um cartão (ver `_template.md`)
`Cabeçalho` (fase, executor, depende de, contexto estimado) → `Objetivo` → `Leia só isto` → `Contexto embutido` → `Arquivos` → `Passos` → `Esqueleto de código` → `Testes a escrever` → `Verificação` → `Não faça` → `Critério de pronto` → `Commit`.

## Executor
| Valor | Significado |
|---|---|
| `agente` | O modelo executa sozinho no repositório |
| `humano` | Rótulo legado para critério que antes dependia de aparelho/hardware/UAC/julgamento visual. Sob o mandato atual, a IA tenta cobri-lo por ADB/harness; se não houver equivalência fiel e segura, registra bloqueio sem pedir execução manual |
| `agente+humano` | Rótulo legado. A IA executa o fluxo inteiro por ADB/harness; se uma parte não puder ser automatizada, não transfere a etapa ao usuário e mantém o critério não aprovado |

**Regra vigente:** nenhum cartão pode exigir que o usuário opere o aparelho/PC, confirme diálogos, faça gesto, tire screenshot ou devolva resultado de teste. Os rótulos antigos não revogam o mandato IA+ADB de `AGENTS.md`; impossibilidade técnica vira bloqueio ou desvio explicitamente aceito, nunca aprovação presumida.

## Convenções de ID
`F<fase>-T<nn>` (ex.: `F1-T02a`). Spikes: `F0-S1`, `F0-S2`, `F0-S3`, `F0-S5`, `F8-S4`. Portão da fase: `F<fase>-GATE`. O arquivo se chama `<ID>.md` dentro de `docs/tasks/fase-<N>/`.

## O que fazer quando faltar um cartão
Por padrão, não improvise cartões. Se o `PROGRESS.md` apontar para **(gerar)**, siga a autorização vigente: o usuário já autorizou a IA a gerar os cartões da Fase 3 a partir do plano **somente depois de F2-GATE aprovada**. Para outras fases sem autorização explícita, peça uma decisão de escopo (não uma execução manual de teste). Todo cartão novo deve obedecer ao mandato IA+ADB.

## Relatórios
Cada fase fecha com `F<N>-GATE`: roda todos os testes, preenche `docs/tests/fase-N.md` (template em `docs/tests/_template.md`) e só então marca a fase como concluída.
