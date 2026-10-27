# F1-GATE — Portão da Fase 1

**Fase:** 1 · **Executor:** agente · **Depende de:** todas as tarefas da Fase 1 · **Tempo:** ~1 h · **Contexto estimado:** ~5k tokens

## Objetivo
Confirmar que a Fase 1 está pronta: protocolo finalizado e sincronizado (Rust/TS), servidor completo, mock-sender documentado, todos os testes verdes, relatório preenchido.

## Leia só isto
`docs/tests/_template.md` e a seção "Checklist de saída" de `docs/phases/fase-1-protocolo-e-rede.md`.

## Passos
1. Rode tudo:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1
   pnpm run vectors:check
   cargo test --release -p lc-protocol --test perf -- --ignored --nocapture
   ```
2. Finalize `docs/protocol.md`: troque o título "(rascunho a ser finalizado na Fase 1)" por "v1.0", confira que ele **concorda** com os vetores e com o CHEATSHEET. Se encontrar divergência, **não decida sozinho**: liste em Observações e pergunte ao usuário.
3. Confira a cobertura dos IDs da Fase 1 em `docs/phases/fase-1-protocolo-e-rede.md` §7: para cada `UT/IT/AT/FT/PT/ST/LT`, aponte o teste (arquivo e nome) ou a medição. Liste em `docs/tests/fase-1.md` qualquer ID **sem** teste (deve ser nenhum).
4. Crie o relatório: `.\scripts\new-report.ps1 -Phase 1` e preencha (resultados, números de PT/LT, evidências: `soak-f1.csv`, saída do `perf`).
5. Revise o `tools/mock-sender/README.md` (todas as flags da Fase 1 documentadas).
6. Atualize `PROGRESS.md`: Fase 1 concluída; **Próxima tarefa: F2-T01 (gerar)**.
7. **Parar** e avisar o usuário: "Fase 1 concluída. Para continuar, peça ao autor do plano para gerar os cartões da Fase 2 (informando o veredito do S3)".

## Verificação
```powershell
Test-Path docs/tests/fase-1.md
Select-String -Path docs/protocol.md -Pattern 'rascunho'
```
Saída esperada: `True`; **nenhuma** ocorrência de "rascunho".

## Critério de pronto
- [ ] CI completo, vetores e `perf` verdes.
- [ ] Relatório **APROVADA**; nenhum ID da Fase 1 sem teste.
- [ ] `protocol.md` final; `PROGRESS.md` atualizado.

## Commit
`chore: fecha a fase 1 (protocolo v1.0, servidor, mock-sender e relatório)`
