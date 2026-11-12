# F2-GATE — Portão da Fase 2

**Fase:** 2 · **Executor:** A · **Depende de:** tarefas em escopo da Fase 2 (F2-T16 excluída por decisão explícita do usuário) · **Tempo:** ~2 h · **Contexto estimado:** ~8k tokens

**Replanejamento ADB-only (2026-11-11, autorizado pelo usuário):** F2-T14 fecha como execução/documentação ADB-only; os casos não observáveis continuam PARCIAL/PENDENTE e não podem ser declarados aprovados. LT-2.01 usa a evidência aprovada existente (120 × 15 s); o usuário dispensou `soak-f2.csv` adicional de 1 s. O GATE deve emitir **APROVADA** somente se os critérios originais estiverem comprovados; caso contrário, registrar **NÃO APROVADA**, os bloqueios precisos e não criar cartões da Fase 3.

**Execução do gate (2026-11-11):** `pnpm run vectors:check` passou (8 frames, 24 JSON) e `:app:testDebugUnitTest` passou (BUILD SUCCESSFUL, 8 s). `scripts/ci.ps1` foi iniciado, mas a pipeline usou `Select-Object -Last 40`, sem saída observável durante a execução; após >1 h, encerrei somente o processo dessa execução. O processo terminou com exit 255 por cancelamento; isso **não** é resultado de teste, portanto CI completo permanece não verificado. Não repetir silenciosamente; se necessário, rodar com logs em tempo real e limite de duração.

**Retomada após autorização de continuidade (2026-11-11):** `scripts/ci.ps1` com saída incremental passou install (1,8 s), lint JS (15,6 s, um warning), typecheck (10,4 s), testes JS (60 testes), cargo fmt/clippy/test (todos verdes) e unitários Android. Lint Android encontrou 11 erros; após correções direcionadas, as verificações reduziram para 2 e depois 1. O único erro restante é `UnsafeOptInUsageError` no uso de `buildPreview` em `CameraXBackend.kt:171`: a anotação `@ExperimentalCamera2Interop` no helper propaga a exigência ao chamador. Duas tentativas de correção já ocorreram; parar conforme `AGENTS.md`, registrar como bloqueio e não marcar CI verde até solução.

**Fechamento da falha de lint (2026-11-11):** com autorização do usuário, o helper recebeu `androidx.annotation.OptIn(markerClass = [ExperimentalCamera2Interop::class])`, conforme a API da dependência; lint Android e unitários passaram. Execução completa subsequente de `scripts/ci.ps1` terminou `CI: OK` em ~100 s, incluindo install, lint JS, typecheck, testes JS, cargo fmt/clippy/test e Android unit+lint. O GATE da **fase** continua NÃO APROVADO devido aos 8 PARCIAIS, 10 PENDENTES e RS-S não integral; não criar F3-T01 ainda.

**Veredito final do GATE (2026-11-11):** `scripts/ci.ps1` concluiu `CI: OK` (install, lint, typecheck, 60 testes JS, `cargo fmt`/clippy/test e Android unit+lint); após uma alteração localizada Android, `scripts/ci.ps1 -Only android` passou novamente. `vectors:check` e unitários passaram. Não houve mudança de protocolo nas alterações F2, então a regressão Rust do servidor do CI satisfaz a reexecução aplicável; o cenário integrado mock→desktop→probe e o fluxo QR/aprovação manual continuam sem evidência completa. A matriz tem 63 IDs únicos: 8 PARCIAIS e 10 PENDENTES; as 20 capturas do catálogo e `docs/devices.md` estão presentes. **Fase 2 NÃO APROVADA**; não gerar F3 até completar ou obter aceitação explícita dos desvios restantes.

**Reabertura (2026-11-11):** o usuário determinou que todos os testes sejam feitos pela IA via ADB, sem execução manual por ele. O veredito acima permanece histórico e NÃO APROVADO; reavaliar somente depois da nova execução T14 sob esse mandato.

**Reavaliação pós-T14 (2026-11-11):** com a T14 fechada (`4628aff`), as verificações do passo 1 foram reexecutadas com saída visível e passaram: `scripts/ci.ps1` terminou `CI: OK` (install, lint, typecheck, 64 testes JS, cargo fmt/clippy/test e Android unit+lint; 1 warning Fast Refresh remanescente), `pnpm run vectors:check` passou (8 frames, 24 JSON) e `:app:testDebugUnitTest` terminou `BUILD SUCCESSFUL`. A matriz consolidada tem 63 IDs únicos: 8 PARCIAIS e 10 PENDENTES; checklist §8 presente (`docs/tests/fase-2.md`, `docs/devices.md`, 20 capturas do catálogo). A RS-S continua incompleta: o cenário integrado mock→desktop→probe com vídeo e a aprovação manual de QR no PC não têm evidência; sem mudança de protocolo em F2, o CI cobre a regressão aplicável do servidor. **Veredito reavaliado: Fase 2 continua NÃO APROVADA** pelos mesmos 8 parciais (AT-2.02, AT-2.04, AT-2.05, AT-2.12, AT-2.13, AT-2.14, FT-2.01, FT-2.12) e 10 pendentes (AT-2.07–2.11, FT-2.02/2.04/2.09/2.14, PT-2.06) e RS-S incompleta; nenhum item parcial/pendente foi promovido sem evidência. F3-T01 continua não aberto.

## Objetivo
Confirmar que a Fase 2 está pronta: celular real pareia (QR → aprovação), reconecta com a chave e transmite H.264 1080p30 por Wi-Fi; os 63 IDs de teste do §7 cobertos; CI completo verde (incluindo a regressão do servidor — RS-S/AT-1.xx); relatório preenchido.

## Leia só isto
`docs/tests/_template.md`. `docs/phases/fase-2-mobile-transmissao-minima.md` §7.8 (regressão) e §8 (checklist de saída).

## Contexto embutido
- Regressão (§7.8): reexecutar **RS-S** e **AT-1.xx** (servidor) após qualquer mudança no protocolo. A Fase 2 **não** muda o protocolo (Kotlin + UI apenas) — o `ci.ps1` completo já reexecuta os testes Rust do `lc-server` (= AT-1.xx). Confirme isso e registre; se encontrar qualquer mudança no protocolo, **pare** e avise o usuário.
- RS-S (`docs/testing.md` §9): `pnpm ci`; mock → desktop → `vcam-probe` (1 min); um fluxo real (QR → aprovação manual → vídeo). O CI cobre a suíte automatizada, mas não equivale ao cenário integrado mock→desktop→probe. O pareamento/vídeo real tem evidência parcial em AT-2.02/IT-2.05; a aprovação manual pelo PC não foi comprovada (IT-2.05 usou harness auto-aprovado). Não declarar RS-S integralmente executada sem essa evidência.
- Os 63 IDs do §7: UT-2.01–2.14 (14) · IT-2.01–2.07 (7) · AT-2.01–2.15 (15) · FT-2.01–2.14 (14) · PT-2.01–2.06 (6) · ST-2.01–2.05 (5) · LT-2.01–2.02 (2).
- Checklist §8: todos os testes obrigatórios aprovados + `docs/devices.md` com ≥ 1 aparelho; capturas do catálogo em `docs/ui/screens/mobile/`; `docs/tests/fase-2.md` preenchido.
- F2-T16 (VPN/rede sem internet) está explicitamente fora deste gate por decisão do usuário; preservar a limitação no relatório.

## Passos
1. Rode tudo:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1
   pnpm run vectors:check
   cd mobile/android; .\gradlew.bat :app:testDebugUnitTest 2>&1 | Select-Object -Last 5; cd ..\..
   ```
2. Cobertura: para cada ID do §7, aponte o teste (arquivo e nome) ou a medição/evidência (tabelas do F2-T14 e F2-T15); liste no relatório qualquer ID **sem** teste (deve ser nenhum; desvios aceitos ficam na seção própria do template).
3. Confira o checklist §8: `docs/devices.md` (≥ 1 aparelho) e capturas em `docs/ui/screens/mobile/`.
4. Preencha/consolide `docs/tests/fase-2.md` com resultados, números PT/LT e evidências existentes. LT-2.01 referencia `lt-201-samsung-soak-20261111.csv` (120 × 15 s); o CSV adicional de 1 s foi dispensado pelo usuário. Não substituir medições por tentativas parciais.
5. Atualize `PROGRESS.md` com o veredito real: Fase 2 concluída somente se todos os critérios originais passarem ou tiverem desvio explicitamente aceito; senão, registrar NÃO APROVADA e os bloqueios, sem criar F3-T01.
6. Se APROVADA, parar e avisar o usuário para pedir ao autor do plano os cartões da Fase 3 (veredito S5: estratégia B — JPEG). Se NÃO APROVADA, reportar os bloqueios sem mascarar status.

## Verificação
```powershell
Test-Path docs/tests/fase-2.md; Test-Path docs/devices.md
(Get-ChildItem docs\ui\screens\mobile\*.png).Count
```
Saída esperada: `True True` e ≥ 1 captura.

## Critério de pronto
- [x] CI completo + vetores + unitários executados; resultados reais registrados, inclusive warnings e a distinção entre CI completo e reteste Android após alteração localizada.
- [x] 63/63 IDs classificados e limitações registradas; veredito NÃO APROVADA pelos 8 PARCIAIS, 10 PENDENTES e RS-S incompleta.
- [x] `PROGRESS.md` atualizado com o veredito; F3-T01 não foi aberta porque a fase não foi aprovada.
- [x] Reavaliar o veredito após a T14 reaberta sob o mandato IA+ADB, sem solicitar testes manuais ao usuário (2026-11-11: CI/vetores/unitários verdes; **NÃO APROVADA** mantida — 8 PARCIAIS, 10 PENDENTES, RS-S incompleta; F3-T01 continua não aberto).

## Commit
`chore: fecha a fase 2 (transmissão mínima no Android, matriz e relatório)`
