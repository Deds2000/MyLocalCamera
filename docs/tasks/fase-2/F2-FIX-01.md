# F2-FIX-01 — Reconexão após queda do Wi-Fi (FT-2.01)

**Fase:** 2 · **Executor:** agente · **Depende de:** — · **Dispositivo principal:** Samsung SM-G781B via ADB

## Passos

1. Reproduzir por ADB o cenário já documentado de 11–12/11: sessão real, modo avião/Wi-Fi desligado e restauração. Coletar somente logcat filtrado de `PrimaryConnection` e `WifiNetwork`; nunca imprimir segredos.
2. Confirmar se novas tentativas ocorrem após o retorno da rede e estabelecer a causa antes de editar código.
3. Corrigir a causa reproduzida. Adicionar teste JVM do laço usando `socketFactory` falso que falha N vezes e caso de rede em cache obsoleta.
4. Reexecutar o cenário por ADB e classificar FT-2.01 na matriz; cleanup seguro de serviço/pareamento temporário.

## Verificação

Executar os testes JVM direcionados e o comando ADB/harness do cenário de reconexão, uma vez cada. O teste deve comprovar reconexão automática dentro do backoff documentado do protocolo. Se não houver caminho fiel/seguro no dispositivo, registrar bloqueado, sem pedir teste manual.

## Critério de pronto

- Causa reproduzida e corrigida, não apenas hipótese.
- Testes JVM cobrem N falhas e rede em cache obsoleta.
- Samsung reconecta sozinho dentro do backoff e FT-2.01 é atualizado com evidência ADB sanitizada.

## Commit

`fix(mobile): reconectar após retorno do Wi-Fi (F2-FIX-01)`
