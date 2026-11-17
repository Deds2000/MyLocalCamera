# F2-FIX-02 — Diagnóstico da latência captura→emissão (PT-2.02)

**Fase:** 2 · **Executor:** agente · **Depende de:** — · **Dispositivo:** Samsung SM-G781B via ADB

## Passos

1. Instrumentar PTS da superfície, saída do `MediaCodec`, entrada na `SendQueue` e escrita no socket. Medir p50/p95 em três execuções comparáveis.
2. Se necessário, testar uma variável por vez entre prioridade/taxa operacional do codec, ausência de B-frames e profundidade de buffers. Antes de usar qualquer chave de `MediaFormat`, confirmar a API e versão na documentação oficial Android.
3. Não alterar o limite de 80 ms. Se p95 não chegar a 80 ms nas três execuções, relatar medições e pedir decisão ao usuário; não declarar conformidade.

## Verificação

Teste JVM/unidade para cálculos e correlação dos estágios; harness instrumentado por ADB no Samsung para as três medições. Registrar amostras agregadas e filtradas, sem segredos.

## Critério de pronto

- Três execuções com p50/p95 por estágio e total, ou bloqueio técnico preciso.
- p95 ≤ 80 ms em cada execução, ou decisão do usuário registrada sem mudar o limite por iniciativa do agente.

## Commit

`perf(mobile): medir latência captura até socket (F2-FIX-02)`
