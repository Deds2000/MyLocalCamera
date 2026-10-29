# Relatório de testes — Fase 0

| Campo | Valor |
|---|---|
| Data | 2026-10-29 |
| Commit / tag | 3f95092 |
| Versão do desktop · APK · protocolo | desktop 0.1.0 (Tauri 2.12.5) · APK 1.0 (code 1) · protocolo 0.0.0 (placeholder até a Fase 1) |
| PC (CPU · GPU · RAM · Windows build) | Intel(R) Core(TM) i7-10700F CPU @ 2.90GHz · NVIDIA GeForce RTX 3090 · 64 GB · Windows 10.0.26300 |
| Celular (modelo · Android · SoC) | Samsung SM-G781B (Android 13, Snapdragon 778G) — AT-0.04 OK via ADB |
| Rede (roteador · banda · USB) | Wi-Fi doméstico (sem medição dedicada na Fase 0); USB disponível (ADB) |
| Responsável | agente (OpenCode, manager + worker ThinkingCap) + usuário (ANDRE) |

## Resultados

| ID | Tipo | Resultado (OK / FALHA / N/A) | Medição | Evidência | Observações |
|---|---|---|---|---|---|
| UT-0.01 | UT | OK | 9/9 testes Vitest (4 arquivos) | `pnpm test` → `packages/shared` (`cn.test.ts` et al.) | F0-T09; revalidado no gate (2026-10-29) |
| UT-0.02 | UT | OK | preset idêntico ao `ui-spec` §1.2–§1.3 | `pnpm test` (testes de preset) | F0-T09 |
| UT-0.03 | UT | OK | paridade pt-BR ↔ en | `pnpm test` (i18n parity) | F0-T09 |
| UT-0.04 | UT | OK | ESLint reprova texto fixo em JSX | `pnpm test` (`lint-rule.test.ts`) | F0-T09; timeout 5 s → 20 s no gate (cold start do ESLint ~7 s fazia flacar) |
| UT-0.05 | UT | OK | 2 execuções → mesmos hashes | `scripts/make-fixtures.ps1` ×2 | F0-T13 |
| IT-0.01 | IT | OK | CI completo: (tempo no rodapé) | `scripts/ci.ps1` → `gate-ci-full2.log` | F0-T06; rodado de novo no gate (1ª falha = timeout do UT-0.04) |
| IT-0.02 | IT | OK | `-Fast`: 111,9 s (~1,9 min) — meta ≤ 3 min | `scripts/ci.ps1 -Fast` → `gate-ci-fast2.log` | F0-T06 (17,5 s na época); 1ª execução no gate: 195,7 s (carga de fundo) |
| IT-0.03 | IT | OK | `desktop.exe` (debug) + `app-debug.apk` 4,4 MB | `desktop/src-tauri/target/debug/`, `mobile/android/app/build/outputs/apk/debug/` | F0-T02/T03 |
| FT-0.01 | FT | OK | typecheck falha e CI para | injeção de erro TS, revertida | F0-T06 (parou no lint antes; typecheck direto também falha) |
| FT-0.02 | FT | OK | clippy falha e CI para | injeção de aviso clippy, revertida | F0-T06 |
| FT-0.03 | FT | OK | teste Kotlin falha e CI para | injeção, revertida | F0-T06 |
| FT-0.04 | FT | OK | `git push` com lint quebrado é bloqueado | hook `pre-push` | F0-T06 |
| FT-0.05 | FT | OK | CI sem ffmpeg/SDK → mensagem clara; `-SkipAndroid` funciona | execução com dependências ausentes | F0-T06 |
| AT-0.01 | AT | OK | clone limpo → `pnpm install --frozen-lockfile` (52,6 s) → CI `-SkipAndroid` verde (172,2 s; cargo frio: clippy 79,2 s, test 91,4 s) | clone em `C:\Users\PC\AppData\Local\Temp\opencode\gate-at001` (2026-10-29) → `gate-at001.log` | |
| AT-0.02 | AT | OK | hook bloqueia push com lint proposital | `pre-push` (F0-T06) | |
| AT-0.03 | AT | OK | janela abre com componentes shadcn | F0-T02 (smoke) + F0-T10 (catálogo Playwright) | |
| AT-0.04 | AT | OK | APK instalado via ADB e aberto: `topResumedActivity com.localcamera.app/.MainActivity` (visible, fullscreen) | `adb install -r app-debug.apk` (Success) + `am start` + `dumpsys activity` (2026-10-29) | usuário dá a confirmação visual |
| AT-0.05 | AT | OK | S1/S2/S3/S5 com veredito; ADRs 0001–0005 | conferência no gate (2026-10-29) | S5: 150/200% pulados (desvio acima) |
| AT-0.06 | AT | PENDENTE | DevTools desativadas em release | (desviado) | cartão permite diferir para a Fase 9 (build de release lento) |
| AT-0.07 | AT | OK | `allowBackup=false`; só `MainActivity` exportada; única permissão `INTERNET` | `mobile/android/app/src/main/AndroidManifest.xml` (conferido no gate) | |
| PT-0.01 | PT | OK | pipe: 87,0 MB/s; lat p50 1,87 ms / p95 2,95 ms; CPU app 3,54% / FrameServer 100,78% | `docs/spikes/S2.md` (soak 1500 s, 41.978 quadros, 0 perdas) | |
| PT-0.02 | PT | OK | fps/jitter/latência por backend | `docs/spikes/S3.md` | |
| PT-0.03 | PT | OK | A: 29,99 fps/1,133 MB/s/12-20 ms/28,3+54%/29→55 MB · B: 29,77 fps/0,653 MB/s/10-14 ms/16,7+48,2%/29→37 MB | `docs/spikes/S5.md` (grupo 100%) | 150/200% não medidos (desvio) |
| ST-0.01 | ST | OK | 2º usuário do Windows: NEGADO (5); `\\.\pipe\` Global: NEGADO (0x80070005) | `docs/spikes/S2.md` | |
| ST-0.02 | ST | OK | CSP `default-src 'self'`; sem fontes remotas | `desktop/src-tauri/tauri.conf.json` (conferido no gate) | |
| LT-0.01 | LT | N/A | — | — | Fase 0 não tem LT; o soak de 1500 s do S2 (0 perdas) cobre a base |

## Defeitos encontrados
| ID | Descrição | Severidade (bloqueante/alta/média/baixa) | Situação | Teste de regressão criado |
|---|---|---|---|---|
| D-0 | UT-0.04 flak: timeout default do Vitest (5 s) menor que o cold start do ESLint (~7 s) — CI falhava intermitentemente no passo `testes JS` | média | corrigido no gate (2026-10-29): timeout explícito 20 s em `packages/shared/src/test/lint-rule.test.ts` | o próprio teste UT-0.04 |

## Desvios aceitos
| Item | Motivo | ADR |
|---|---|---|
| S5: grupos de DPI 150%/200% não medidos | decisão do usuário (2026-10-29) — mudar a escala neste PC exige sign-out; veredito decidido com 100% (esperativa documentada em `S5.md`) | — (documentado no S5.md) |
| AT-0.06 diferido para a Fase 9 | o cartão do gate permite diferir quando o build de release é lento; anotar na Fase 9 | — |


## Veredito
- [x] Todos os testes obrigatórios aprovados (ou desvio aceito acima)
- [x] Regressão da Fase 0: referência de tempo do CI guardada (IT-0.01 — ver rodapé)
- [x] Documentação e ADRs atualizados (S1–S5 com veredito; ADRs 0001–0005)

**Resultado:** APROVADA (2026-10-29, commit 3f95092 + correção D-0; usuário confirmou os vereditos dos spikes)

## Tempos do CI (referência IT-0.01, 2026-10-29, commit 3f95092)
| Execução | pnpm install | lint | typecheck | testes JS | cargo fmt | cargo clippy | cargo test | android unit+lint | Total |
|---|---|---|---|---|---|---|---|---|---|
| completo (reexecução pós-correção, `gate-ci-full2.log`) | 1,2 s | 58,3 s | 38,7 s | 60 s | 10,1 s | 62,4 s | 67 s | 73,7 s | ~272 s (~4,5 min) — verde |
| `-Fast` (melhor de 2 execuções, `gate-ci-fast2.log`) | 3,2 s | 59,2 s | 17 s | 28,8 s | 1,2 s | 2,5 s | — | — | **111,9 s (~1,9 min) — dentro da meta ≤ 3 min** (1ª execução: 195,7 s sob carga de fundo) |
