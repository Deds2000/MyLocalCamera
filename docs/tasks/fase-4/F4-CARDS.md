# F4-CARDS — Gerar cartões da Fase 4

**Fase:** 4 · **Executor:** agente · **Depende de:** —

## Objetivo

Gerar em `docs/tasks/fase-4/` os cartões da fase, alinhados à especificação `docs/phases/fase-4-camera-virtual.md`, às decisões já aprovadas nos ADRs e ao fluxo do `AGENTS.md`.

## Critério de pronto

- DAG sem ciclos e cada cartão com no máximo três dependências diretas.
- 100% dos IDs de teste da Fase 4 mapeados a cartões e critérios verificáveis.
- Executor, passos, verificação, critério de pronto e mensagem de commit por cartão.
- Nenhuma decisão arquitetural nova inferida pelo agente; lacunas são perguntadas antes de implementar.

## Verificação

Auditar todos os IDs de testes da especificação por grep; conferir dependências em grafo (sem ciclos e máximo 3) e validar que todo link da tabela do `PROGRESS.md` existe.

## Commit

`docs: gerar cartões da Fase 4 (F4-CARDS)`
