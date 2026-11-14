# PROGRESS.md — andamento do LocalCamera

> Legenda: `[ ]` a fazer · `[~]` em andamento · `[x]` pronta · `[!]` bloqueada. Executor: agente; testes de aparelho via IA + ADB, conforme `AGENTS.md`.

**Próxima tarefa:** **F2-FIX-01** — reconexão após queda de Wi-Fi. **D1:** sequência intercalada autorizada. **Atualizado:** 2026-11-13.

## Bloqueios abertos

- **Fase 2: NÃO APROVADA.** Veredito preservado; autorização de 2026-11-12 permite iniciar a Fase 3. Reavaliar somente após F2-FIX-01…04 em F2-GATE-R.
- **D7 aprovada (2026-11-13):** canal limitado de quadros e comandos, descarte de não-chave mais antigos, sinalização de keyframe necessário e escritor único por sessão; sem mudança do protocolo v1.0.
- **Fase 2:** FT-2.01 reconexão segue parcial; PT-2.02 excedeu 80 ms historicamente e está como desvio aceito. F2-FIX-01/02 reabrem esses pontos sem mudar o limite.
- **Decisões futuras:** D2 após F2-FIX-02; D3 antes de instalar `cargo audit`/`cargo deny`; D4 antes de remover `LocalCamera-docs.zip`; D6 antes de aceitar desvios no gate.
- O registro antigo indica F3-T14 (agregador) e F3-GATE sem cartões; a proposta atual os trata como existentes. Reconciliar essa divergência antes de fechar o gate, sem inventar cartões nesta tarefa.
- Histórico e observações anteriores: [`docs/progress-archive/PROGRESS-2026-11-13.md`](docs/progress-archive/PROGRESS-2026-11-13.md). D5: nota do servidor local de LLM removida por decisão do usuário.

## Sequência e cartões

| St | ID | Resumo | Depende de | Cartão |
|---|---|---|---|---|
| [x] | R-01 | Compactar progresso e arquivar histórico | — | `docs/tasks/R-01.md` |
| [x] | R-05 | Dividir conn/handshake sem mudar comportamento; 107 testes passaram, 1 ignorado; clippy/fmt verdes | R-01 | `docs/tasks/R-05.md` |
| [x] | F3-T13b | Quadros limitados e comandos no lc-server; PONG, ordem/PTS, keyframe e sessão encerrada verificados | R-05 | `docs/tasks/fase-3/F3-T13b.md` |
| [ ] | F2-FIX-01 | Reconexão após queda de Wi-Fi | — | `docs/tasks/fase-2/F2-FIX-01.md` |
| [ ] | F3-T13 | Ponte Tauri ↔ lc-server | T01, T13b | `docs/tasks/fase-3/F3-T13.md` |
| [ ] | F2-FIX-03 | Testes JVM do estado da câmera | — | `docs/tasks/fase-2/F2-FIX-03.md` |
| [ ] | F3-T07 | Decodificador H.264 | F3-T13b, F3-T13 | `docs/tasks/fase-3/F3-T07.md` |
| [ ] | F2-FIX-02 | Medição de latência captura→socket | — | `docs/tasks/fase-2/F2-FIX-02.md` |
| [ ] | F3-T14a | Store Zustand | F3-T13 | `docs/tasks/fase-3/F3-T14a.md` |
| [ ] | F3-T04 | Estado vazio e aba Conexão | T02, T14b, T14a | `docs/tasks/fase-3/F3-T04.md` |
| [ ] | F3-T05 | ApprovalDialog | T02, T14b, T14a | `docs/tasks/fase-3/F3-T05.md` |
| [ ] | F3-T06 | Dispositivos confiáveis | T04 | `docs/tasks/fase-3/F3-T06.md` |
| [ ] | F3-T08 | Composição NV12 | T07 | `docs/tasks/fase-3/F3-T08.md` |
| [ ] | F3-T09 | Preview JPEG/canvas | T08, T14a | `docs/tasks/fase-3/F3-T09.md` |
| [ ] | F2-FIX-04 | Reclassificar IDs Fase 2 sem evidência | F2-FIX-01 | `docs/tasks/fase-2/F2-FIX-04.md` |
| [ ] | F2-GATE-R | Reavaliar gate da Fase 2 | FIX-02, FIX-03, FIX-04 | `docs/tasks/fase-2/F2-GATE-R.md` |
| [ ] | F3-T10 | Regras de desempenho | T09 | `docs/tasks/fase-3/F3-T10.md` |
| [ ] | F3-T11 | Estados do preview e toolbar | T10 | `docs/tasks/fase-3/F3-T11.md` |
| [ ] | F3-T14a2 | Painel lateral | T04, T11, T14b | `docs/tasks/fase-3/F3-T14a2.md` |
| [ ] | F3-T12 | HealthBar | T13, T14a, T02 | `docs/tasks/fase-3/F3-T12.md` |
| [ ] | F3-T14c | Catálogo, capturas e axe | T14a2, T12, T05 | `docs/tasks/fase-3/F3-T14c.md` |
| [ ] | F3-T17 | Instrumentação/soaks de desempenho | T10, T14a2 | `docs/tasks/fase-3/F3-T17.md` |
| [ ] | R-02 | README e raiz do repositório | R-01 | `docs/tasks/R-02.md` |
| [ ] | R-03 | CI hospedado Windows | R-02 | `docs/tasks/R-03.md` |
| [ ] | R-04 | Segurança e threat model LAN | — | `docs/tasks/R-04.md` |
| [ ] | F4-CARDS | Gerar cartões da Fase 4 | — | `docs/tasks/fase-4/F4-CARDS.md` |
| [ ] | F4-T01 | DLL de câmera virtual com quadros sintéticos | F4-CARDS | `docs/tasks/fase-4/F4-T01.md` |
| [x] | F3-T01 | Janela, titlebar e layout | — | `docs/tasks/fase-3/F3-T01.md` |
| [x] | F3-T02 | Componentes base e animações | T01 | `docs/tasks/fase-3/F3-T02.md` |
| [x] | F3-T03 | i18n `common.*` | — | `docs/tasks/fase-3/F3-T03.md` |
| [x] | F3-T14b | i18n `desktop.*` | T03 | `docs/tasks/fase-3/F3-T14b.md` |

## Evidência recente

- **R-01:** histórico arquivado; 127 IDs de tarefa e 111 IDs de teste distintos preservados; nota removida por D5; Fase 2 segue NÃO APROVADA.
- **R-05:** `cargo test -p lc-server` — 107 aprovados, 1 ignorado; `cargo clippy -p lc-server` e `cargo fmt --all --check` verdes. Testes existentes não editados; módulos extraídos com ≤300 linhas.
- **F3-T13b:** `cargo test -p lc-server`, `cargo clippy -p lc-server`, `cargo test -p mock-sender` e `cargo fmt --all --check` verdes; novas integrações cobrem pressão sem bloquear PONG, metadados/ordem, envio de SET/keyframe e erro após encerramento. Protocolo v1.0 e `lc-protocol` sem alterações.
