# F0-GATE — Portão da Fase 0

**Fase:** 0 · **Executor:** agente+humano · **Depende de:** todas as tarefas da Fase 0 · **Tempo:** ~1 h · **Contexto estimado:** ~5k tokens

## Objetivo
Confirmar que a Fase 0 está **realmente** pronta: CI verde, esqueletos abrindo, spikes com veredito e ADR, relatório de testes preenchido.

## Leia só isto
`docs/tests/_template.md` (campos do relatório) e a seção "Checklist de saída" de `docs/phases/fase-0-fundacao-e-spikes.md`.

## Passos
1. Rode o CI completo e o rápido; anote os tempos:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1 -Fast
   ```
2. Confirme os testes da Fase 0 (`UT-0.01…UT-0.04`, `IT-0.01…IT-0.03`, `FT-0.01…FT-0.05`) e os ATs: `AT-0.01` (CI numa pasta limpa: clone para uma pasta temporária e rode `pnpm install --frozen-lockfile` + `scripts/ci.ps1 -SkipAndroid`), `AT-0.02` (hook), `AT-0.03` (desktop abre), `AT-0.04` (**humano**: APK abre no celular), `AT-0.05` (spikes), `AT-0.06` (DevTools desativadas em release: `pnpm --filter desktop tauri build` e abrir o instalador/binário — pode ser marcado como pendente para a Fase 9 se o build de release for lento, anotando em Observações), `AT-0.07` (manifesto Android).
3. Crie o relatório: `.\scripts\new-report.ps1 -Phase 0` e preencha a tabela de resultados (OK/FALHA, medições, evidências).
4. Verifique a existência de `docs/spikes/S1.md`, `S2.md`, `S3.md`, `S5.md` e dos ADRs 0001–0005, cada spike com **veredito não vazio**.
5. Atualize `PROGRESS.md`: marque a Fase 0 como concluída, aponte **Próxima tarefa: F1-T01** e liste em Observações qualquer desvio aceito.
6. **[Humano]** Leia o veredito dos spikes e confirme que concorda com as decisões (principalmente S1 e S3). Se algum spike for *inviável* sem plano B, **pare**: a Fase 1 não começa.

## Verificação
```powershell
Get-ChildItem docs/spikes/S*.md | ForEach-Object { $_.Name + ' : ' + ((Select-String -Path $_.FullName -Pattern 'Veredito' | Select-Object -First 1).Line) }
Get-ChildItem docs/adr/*.md | Select-Object -ExpandProperty Name
Test-Path docs/tests/fase-0.md
```
Saída esperada: 4 spikes com veredito; ADRs 0000–0005; `True`.

## Critério de pronto
- [ ] CI completo e `-Fast` verdes (tempos registrados).
- [ ] Relatório `docs/tests/fase-0.md` preenchido e **APROVADA**.
- [ ] Spikes e ADRs completos; usuário confirmou os vereditos.
- [ ] `PROGRESS.md` atualizado.

## Commit
`chore: fecha a fase 0 (relatório de testes e vereditos dos spikes)`
