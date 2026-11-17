# F2-FIX-03 — Testes JVM do estado da câmera

**Fase:** 2 · **Executor:** agente · **Depende de:** —

## Passos

1. Extrair de `CameraXBackend` a regra pura que traduz estado CameraX em evento/estado do app; manter o comportamento de produção.
2. Cobrir: `CLOSED` inicial antes de `OPEN` é ignorado; `CLOSED` após `OPEN` é falha; câmera já em uso por outro app é reportada corretamente.
3. Executar smoke direcionado por ADB no Samsung e confirmar que o comportamento do aparelho não mudou.

## Verificação

Executar os testes JVM direcionados e o smoke ADB uma vez cada. Se o smoke não tiver caminho seguro disponível, registrar o bloqueio sem pedir operação manual.

## Critério de pronto

- Regra pura coberta pelos três casos, sem alterar semântica do app.
- Smoke ADB confirma comportamento inalterado; teste e lint direcionados verdes.

## Commit

`test(mobile): cobrir transições de estado da câmera (F2-FIX-03)`
