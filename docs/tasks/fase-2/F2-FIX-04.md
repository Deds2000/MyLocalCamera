# F2-FIX-04 — Reclassificar os IDs da Fase 2 sem evidência

**Fase:** 2 · **Executor:** agente · **Depende de:** F2-FIX-01

## Escopo

Revisar AT-2.06 a AT-2.10, FT-2.02, FT-2.04, FT-2.09, FT-2.14 e PT-2.06. FT-2.10/FT-2.11 permanecem adiados conforme decisão anterior; F2-T16 permanece fora do gate.

## Passos

Para cada um dos dez IDs, escolher e justificar uma categoria:

1. Automatizável com ADB ou harness: executar por agente, anexar evidência segura e atualizar `docs/tests/fase-2.md`.
2. Dependente de desktop real: vincular ao gate da Fase 3 ou 4, sem marcar como aprovado antes da evidência.
3. Exige ação física/rede externa: propor desvio explícito, sem pedir que o usuário opere aparelho/PC ou colete evidência.

Reutilizar evidência aprovada da versão exata; não repetir soaks cobertos.

## Verificação

Conferir por grep que todos os dez IDs aparecem uma vez na tabela atualizada com evidência ou desvio/pendência justificado; F2-T16 e FT-2.10/2.11 não voltam ao gate.

## Critério de pronto

- Os dez IDs classificados individualmente na matriz, cada um com evidência anexada, destino de gate ou desvio proposto.
- Nenhum item manual foi tratado como aprovado por simulação não equivalente.

## Commit

`docs(testes): reclassificar gaps da Fase 2 (F2-FIX-04)`
