# F2-GATE — Portão da Fase 2

**Fase:** 2 · **Executor:** A · **Depende de:** todas as tarefas da Fase 2 · **Tempo:** ~2 h · **Contexto estimado:** ~8k tokens

## Objetivo
Confirmar que a Fase 2 está pronta: celular real pareia (QR → aprovação), reconecta com a chave e transmite H.264 1080p30 por Wi-Fi; os 63 IDs de teste do §7 cobertos; CI completo verde (incluindo a regressão do servidor — RS-S/AT-1.xx); relatório preenchido.

## Leia só isto
`docs/tests/_template.md`. `docs/phases/fase-2-mobile-transmissao-minima.md` §7.8 (regressão) e §8 (checklist de saída).

## Contexto embutido
- Regressão (§7.8): reexecutar **RS-S** e **AT-1.xx** (servidor) após qualquer mudança no protocolo. A Fase 2 **não** muda o protocolo (Kotlin + UI apenas) — o `ci.ps1` completo já reexecuta os testes Rust do `lc-server` (= AT-1.xx). Confirme isso e registre; se encontrar qualquer mudança no protocolo, **pare** e avise o usuário.
- RS-S (`docs/testing.md` §9): `pnpm ci`; mock → desktop → `vcam-probe` (1 min); um fluxo real (QR → aprovação → vídeo). O CI completo cobre o primeiro item; o fluxo real já foi IT-2.05/AT-2.02 — registre o mapeamento no relatório.
- Os 63 IDs do §7: UT-2.01–2.14 (14) · IT-2.01–2.07 (7) · AT-2.01–2.15 (15) · FT-2.01–2.14 (14) · PT-2.01–2.06 (6) · ST-2.01–2.05 (5) · LT-2.01–2.02 (2).
- Checklist §8: todos os testes obrigatórios aprovados + `docs/devices.md` com ≥ 1 aparelho; capturas do catálogo em `docs/ui/screens/mobile/`; `docs/tests/fase-2.md` preenchido.

## Passos
1. Rode tudo:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1
   pnpm run vectors:check
   cd mobile/android; .\gradlew.bat :app:testDebugUnitTest 2>&1 | Select-Object -Last 5; cd ..\..
   ```
2. Cobertura: para cada ID do §7, aponte o teste (arquivo e nome) ou a medição/evidência (tabelas do F2-T14 e F2-T15); liste no relatório qualquer ID **sem** teste (deve ser nenhum; desvios aceitos ficam na seção própria do template).
3. Confira o checklist §8: `docs/devices.md` (≥ 1 aparelho) e capturas em `docs/ui/screens/mobile/`.
4. Crie o relatório: `.\scripts\new-report.ps1 -Phase 2` e preencha (resultados, números de PT/LT, evidências: `soak-f2.csv`, screenshots, logs do encoder).
5. Atualize `PROGRESS.md`: Fase 2 concluída; **Próxima tarefa: F3-T01 (gerar)**.
6. **Parar** e avisar o usuário: "Fase 2 concluída. Para continuar, peça ao autor do plano para gerar os cartões da Fase 3 (veredito S5: estratégia B — JPEG)".

## Verificação
```powershell
Test-Path docs/tests/fase-2.md; Test-Path docs/devices.md
(Get-ChildItem docs\ui\screens\mobile\*.png).Count
```
Saída esperada: `True True` e ≥ 1 captura.

## Critério de pronto
- [ ] CI completo + vetores + unitários verdes; regressão confirmada e registrada.
- [ ] 63/63 IDs cobertos (ou desvio aceito documentado); relatório **APROVADA**.
- [ ] `PROGRESS.md` atualizado (Fase 3 aguardando cartões).

## Commit
`chore: fecha a fase 2 (transmissão mínima no Android, matriz e relatório)`
