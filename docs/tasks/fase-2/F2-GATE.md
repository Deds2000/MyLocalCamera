# F2-GATE — Portão da Fase 2

**Fase:** 2 · **Executor:** A · **Depende de:** tarefas em escopo da Fase 2 (F2-T16 excluída por decisão explícita do usuário) · **Tempo:** ~2 h · **Contexto estimado:** ~8k tokens

**Replanejamento ADB-only (2026-11-11, autorizado pelo usuário):** F2-T14 fecha como execução/documentação ADB-only; os casos não observáveis continuam PARCIAL/PENDENTE e não podem ser declarados aprovados. LT-2.01 usa a evidência aprovada existente (120 × 15 s); o usuário dispensou `soak-f2.csv` adicional de 1 s. O GATE deve emitir **APROVADA** somente se os critérios originais estiverem comprovados; caso contrário, registrar **NÃO APROVADA**, os bloqueios precisos e não criar cartões da Fase 3.

**Execução do gate (2026-11-11):** `pnpm run vectors:check` passou (8 frames, 24 JSON) e `:app:testDebugUnitTest` passou (BUILD SUCCESSFUL, 8 s). `scripts/ci.ps1` foi iniciado, mas a pipeline usou `Select-Object -Last 40`, sem saída observável durante a execução; após >1 h, encerrei somente o processo dessa execução. O processo terminou com exit 255 por cancelamento; isso **não** é resultado de teste, portanto CI completo permanece não verificado. Não repetir silenciosamente; se necessário, rodar com logs em tempo real e limite de duração.

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
- [ ] CI completo + vetores + unitários executados; registrar os resultados reais, inclusive falhas.
- [ ] 63/63 IDs classificados e limitações registradas; emitir APROVADA somente se os critérios obrigatórios forem satisfeitos, caso contrário NÃO APROVADA com os bloqueios explícitos.
- [ ] `PROGRESS.md` atualizado com o veredito; não abrir F3-T01 se a fase não for aprovada.

## Commit
`chore: fecha a fase 2 (transmissão mínima no Android, matriz e relatório)`
