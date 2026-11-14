# F2-GATE-R — Reavaliar o gate da Fase 2

**Fase:** 2 · **Executor:** agente · **Depende de:** F2-FIX-02, F2-FIX-03, F2-FIX-04

> F2-FIX-04 depende de F2-FIX-01; essa dependência transitiva mantém o DAG dentro do máximo de três dependências diretas.

## Passos

1. Reunir evidência e matriz atualizadas das F2-FIX-01 a 04, sem repetir evidência já aprovada e ainda equivalente.
2. Produzir novo relatório do gate, separando aprovado, parcial, pendente, desvio aceito e fora de escopo.
3. Não aprovar a fase por conta própria: apresentar o relatório ao usuário para decisão entre aprovar, aprovar com desvios ou manter pendente até F3-GATE.

## Verificação

Auditar que cada critério de gate tem referência verificável e que limitações/desvios não foram convertidos em aprovação sem autorização. Gate permanece pendente até decisão do usuário.

## Critério de pronto

- Relatório completo e rastreável entregue; veredito só muda após decisão explícita do usuário.

## Commit

`docs(testes): reavaliar gate da Fase 2 (F2-GATE-R)`
