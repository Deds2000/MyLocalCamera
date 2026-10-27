# PROGRESS.md — andamento do LocalCamera

> O agente atualiza este arquivo ao fim de **cada** tarefa. Legenda: `[ ]` a fazer · `[~]` em andamento · `[x]` pronta · `[!]` bloqueada.
> Executor: **A** = agente · **H** = humano (o agente para e entrega o roteiro) · **A+H** = agente prepara, humano executa a parte manual.

**Próxima tarefa:** `F0-T05`  ·  **Fase atual:** 0  ·  **Última atualização:** 2026-10-27

## Bloqueios
(nenhum)

## Observações
(o agente anota aqui problemas fora do escopo da tarefa atual)
- F0-T04 (resolvido em F0-T03): `typescript` do `packages/shared` foi alinhado para `~6.0.3`; a versão 7.0.2 duplicava os tipos do i18next e quebrava o typecheck do mobile.
- F0-T02: shadcn/ui criado manualmente (CLI que geraria Tailwind 4 foi pulada); `baseUrl` removido do tsconfig por TS5101; chave extra i18n `desktop.smoke.close` para o label do X do Dialog.
- F0-T03: `@capacitor/app` estável existe só até 8.1.2; core/cli/android em 8.5.3, peer >=8 (compatível).
- F0-T03: i18n passou a usar `I18nextProvider` em desktop e mobile; sem isso o APK mostrava as chaves literais.
- F0-T03: APK instalado e validado por screenshot no Samsung SM-G781B; Moto G5 (API 27) ainda não testado.
- F0-T14: `new-report.ps1` lê o template como UTF-8 e grava sem BOM; sem isso o PowerShell 5.1 corrompia acentos e o separador `·`.

---

## Fase 0 — Fundação, testes e spikes  → `docs/phases/fase-0-fundacao-e-spikes.md`
Ordem de execução (respeite as dependências):

| St | ID | Título | Exec | Dep. | Cartão |
|---|---|---|---|---|---|
| [x] | F0-T01 | Monorepo pnpm, ESLint, Prettier, tsconfig | A | — | `docs/tasks/fase-0/F0-T01.md` |
| [x] | F0-T07 | Estrutura de docs, ADR 0001, scripts de relatório | A | T01 | `docs/tasks/fase-0/F0-T07.md` |
| [x] | F0-T04 | Pacote `@localcamera/shared` (cn, preset, tokens, i18n) | A | T01 | `docs/tasks/fase-0/F0-T04.md` |
| [x] | F0-T02 | App desktop: Tauri 2 + React 19 + Tailwind 3.4 + shadcn | A | T04 | `docs/tasks/fase-0/F0-T02.md` |
| [x] | F0-T03 | App mobile: Capacitor + React 19 + Tailwind 3.4 + shadcn | A+H | T04 | `docs/tasks/fase-0/F0-T03.md` |
| [ ] | F0-T05 | Workspace Rust (lc-protocol, mock-sender, toolchain) | A | T01 | `docs/tasks/fase-0/F0-T05.md` |
| [ ] | F0-T09 | Infra de testes TypeScript (Vitest, RTL, testes UT-0.01–04) | A | T04 | `docs/tasks/fase-0/F0-T09.md` |
| [ ] | F0-T12 | Infra de testes Rust (`lc-testkit`, proptest, tokio) | A | T05 | `docs/tasks/fase-0/F0-T12.md` |
| [ ] | F0-T11 | Infra de testes Kotlin/Android (JUnit, coroutines-test) | A+H | T03 | `docs/tasks/fase-0/F0-T11.md` |
| [ ] | F0-T06 | CI local (`scripts/ci.ps1`) e hook de `pre-push` | A | T09, T12 | `docs/tasks/fase-0/F0-T06.md` |
| [ ] | F0-T13 | Fixtures: `make-fixtures.ps1` e `fixture-tool` | A | T12 | `docs/tasks/fase-0/F0-T13.md` |
| [x] | F0-T14 | Relatórios de teste (`new-report.ps1`) | A | T07 | `docs/tasks/fase-0/F0-T14.md` |
| [ ] | F0-T10 | (opcional) Playwright para o catálogo de componentes | A | T02, T03 | `docs/tasks/fase-0/F0-T10.md` |
| [ ] | F0-S1 | Spike S1 — câmera virtual × NVIDIA Broadcast | A+H | T01 | `docs/tasks/fase-0/F0-S1.md` |
| [ ] | F0-S2 | Spike S2 — canal Frame Server ↔ app | A+H | S1 | `docs/tasks/fase-0/F0-S2.md` |
| [ ] | F0-S3 | Spike S3 — CameraX × Camera2 (e câmeras simultâneas) | A+H | T03 | `docs/tasks/fase-0/F0-S3.md` |
| [ ] | F0-S5 | Spike S5 — estratégia de preview (WebCodecs × JPEG do Rust) | A+H | T02, T13 | `docs/tasks/fase-0/F0-S5.md` |
| [ ] | F0-GATE | Portão da Fase 0 (todos os testes e relatório) | A+H | todas | `docs/tasks/fase-0/F0-GATE.md` |

## Fase 1 — Protocolo e rede  → `docs/phases/fase-1-protocolo-e-rede.md`
Ordem de execução (respeite as dependências). A lógica de rede fica na crate `crates/lc-server` (sem Tauri).

| St | ID | Título | Exec | Dep. | Cartão |
|---|---|---|---|---|---|
| [ ] | F1-T01 | Vetores de teste do protocolo (`protocol-vectors.json`) | A | F0-GATE | `docs/tasks/fase-1/F1-T01.md` |
| [ ] | F1-T02a | `lc-protocol`: cabeçalho e `FrameReader` | A | T01 | `docs/tasks/fase-1/F1-T02a.md` |
| [ ] | F1-T02b | `lc-protocol`: mensagens JSON (serde) e `Message` | A | T02a | `docs/tasks/fase-1/F1-T02b.md` |
| [ ] | F1-T03 | Tipos/validadores TS + URI de pareamento (+ vetores de URI) | A | T01 | `docs/tasks/fase-1/F1-T03.md` |
| [ ] | F1-T04a | `lc-server`: listener, limites e laço de leitura | A | T02b | `docs/tasks/fase-1/F1-T04a.md` |
| [ ] | F1-T05 | Confiança: chaves, hashes, armazenamento atômico | A | T04a | `docs/tasks/fase-1/F1-T05.md` |
| [ ] | F1-T09a | Tickets de áudio (`TicketStore`, lógica pura) | A | T05 | `docs/tasks/fase-1/F1-T09a.md` |
| [ ] | F1-T04b | Handshake, autenticação, versão e sessão única | A | T04a, T05, T09a | `docs/tasks/fase-1/F1-T04b.md` |
| [ ] | F1-T06 | Fluxo de aprovação (`PENDING`, timeout, decisão) | A | T04b | `docs/tasks/fase-1/F1-T06.md` |
| [ ] | F1-T07 | Janela de aprovação USB (15 s, localhost) | A | T06 | `docs/tasks/fase-1/F1-T07.md` |
| [ ] | F1-T15 | Limitação de falhas de auth e de taxa de conexões | A | T04b | `docs/tasks/fase-1/F1-T15.md` |
| [ ] | F1-T09b | Socket de áudio (`AUDIO_HELLO`, contadores) | A | T04b, T09a | `docs/tasks/fase-1/F1-T09b.md` |
| [ ] | F1-T10 | Keepalive, RTT e agregador de estatísticas (500 ms) | A | T09b | `docs/tasks/fase-1/F1-T10.md` |
| [ ] | F1-T16 | Logs com redação de segredos | A | T04b | `docs/tasks/fase-1/F1-T16.md` |
| [ ] | F1-T11a | `mock-sender`: pareamento, reconexão, vídeo/áudio | A | T09b, F0-T13 | `docs/tasks/fase-1/F1-T11a.md` |
| [ ] | F1-T11b | `mock-sender`: flags de falha + AT/FT automatizados | A | T11a, T10 | `docs/tasks/fase-1/F1-T11b.md` |
| [ ] | F1-T13 | Integração, desempenho e *soak* (cria `soak.ps1`) | A+H | T11b, T15, T16, T07 | `docs/tasks/fase-1/F1-T13.md` |
| [ ] | F1-GATE | Portão da Fase 1 (protocolo v1.0 e relatório) | A | todas | `docs/tasks/fase-1/F1-GATE.md` |

## Fase 2 — Mobile: transmissão mínima  → `docs/phases/fase-2-mobile-transmissao-minima.md`
**Cartões: (gerar)** — depende do veredito do S3. Tarefas previstas:
`F2-T01` Capacitor + deep link · `F2-T03` telas e roteamento · `F2-T03a` componentes e tokens · `F2-T03b` textos i18n · `F2-T03c` catálogo · `F2-T04` `ProtocolCodec.kt` · `F2-T05` `PairingStore.kt` · `F2-T05a` parser da URI · `F2-T06` `CapabilitiesProbe` · `F2-T07` `CameraBackend` · `F2-T08` `VideoEncoder.kt` · `F2-T09` `PrimaryConnection.kt` · `F2-T10` `SendQueue.kt` · `F2-T11` `StateReducer.kt` · `F2-T12` `StreamingService` · `F2-T13` `LocalCameraPlugin.kt` · `F2-T16` rede do Android · `F2-T14` testes · `F2-T15` relatório · `F2-GATE`

## Fase 3 — Desktop: UI, decodificação e preview  → `docs/phases/fase-3-desktop-ui-e-preview.md`
**Cartões: (gerar)** — depende do veredito do S5. Tarefas previstas:
`F3-T01` janela e titlebar · `F3-T02` componentes e animações · `F3-T03` i18n · `F3-T13` comandos/eventos Tauri · `F3-T14a` store · `F3-T04` estado vazio e aba Conexão · `F3-T05` `ApprovalDialog` · `F3-T06` confiáveis · `F3-T07` decodificador · `F3-T08` composição + `composition.md` · `F3-T09` preview · `F3-T10` regras de desempenho · `F3-T11` chips e barra de ferramentas · `F3-T14a2` painel lateral · `F3-T12` `HealthBar` · `F3-T14c` catálogo · `F3-T17` instrumentação de latência · `F3-T14` testes · `F3-GATE`

## Fase 4 — Câmera virtual  → `docs/phases/fase-4-camera-virtual.md`
**Cartões: (gerar)** — depende dos vereditos S1 e S2. Tarefas previstas:
`F4-T01` DLL · `F4-T02` cliente do canal · `F4-T03` placeholder · `F4-T04` módulo `vcam/` · `F4-T05` indicador · `F4-T06` `dev-setup.ps1` · `F4-T07` assistente · `F4-T08` plano B DirectShow (se S1 mandar) · `F4-T09` testes longos · `F4-T10` README · `F4-T11` métricas · `F4-T12` `vcam-probe` · `F4-GATE` (**tag `v0.1.0`**)

## Fase 5 — USB e descoberta  → `docs/phases/fase-5-usb-e-descoberta.md`
**Cartões: (gerar).** `F5-T01` localizar adb · `F5-T02` rastrear aparelhos · `F5-T03` fluxo Conectar · `F5-T04` UI · `F5-T05` ciclo de vida · `F5-T11` auto-conectar · `F5-T06` anúncio mDNS · `F5-T07` firewall · `F5-T08` `Discovery.kt` · `F5-T09` cadeia de reconexão · `F5-T10` PCs conhecidos · `F5-T12` `fake-adb` · `F5-T13` testes · `F5-GATE`

## Fase 6 — Controle bidirecional  → `docs/phases/fase-6-controle-bidirecional.md`
**Cartões: (gerar).** `F6-T01` reducer · `F6-T02` reconfiguração · `F6-T03` otimismo/reconciliação · `F6-T04` espelho/rotação · `F6-T05` aba Câmera · `F6-T06` tela Transmitindo · `F6-T07` STATS + ABR · `F6-T08` keyframe · `F6-T09` indicadores · `F6-T10` persistência · `F6-T11` `--emulate-phone` · `F6-T12` testes · `F6-GATE` (**tag `v0.2.0`**)

## Fase 7 — Câmera dupla  → `docs/phases/fase-7-camera-dupla.md`
**Cartões: (gerar)** — depende do S3. `F7-T01…T17` · `F7-GATE` (**tag `v0.3.0`**)

## Fase 8 — Áudio  → `docs/phases/fase-8-audio.md`
**Cartões: (gerar)** — começa pelo spike S4. `F8-S4a/b/c` · `F8-T01…T15` · `F8-GATE` (**tag `v0.4.0`**)

## Fase 9 — Polimento e instalação  → `docs/phases/fase-9-polimento-e-instalacao.md`
**Cartões: (gerar).** `F9-T01…T17` · `F9-GATE` (**tag `v1.0.0`**)
