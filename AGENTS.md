# AGENTS.md — LocalCamera

Projeto: celular Android vira webcam e microfone para Windows 11 (Wi-Fi ou USB). Monorepo pnpm + Rust + Kotlin + C++.
**Você tem pouco contexto. Trabalhe em passos pequenos e não abra documentos à toa.**

## Fluxo obrigatório (siga SEMPRE, uma tarefa por vez)
1. Abra `PROGRESS.md`. Pegue a **primeira tarefa `[ ]`** cuja dependência já está `[x]`.
2. Abra **somente o cartão dela**: `docs/tasks/fase-N/<ID>.md`. Ele traz tudo o que você precisa. Só abra outro arquivo se o cartão mandar.
3. Se o campo **Executor** for `humano`, **PARE** e mostre ao usuário o passo a passo do cartão. Não finja que testou.
4. Execute os passos na ordem. Depois de cada bloco, rode o comando de **verificação** do cartão.
5. Falhou 3 vezes no mesmo ponto? **Pare**, escreva em `PROGRESS.md` (seção *Bloqueios*) o erro e o que tentou, e pergunte ao usuário.
6. Cumpriu o **Critério de pronto**? Faça o commit (mensagem sugerida no cartão), marque `[x]` em `PROGRESS.md` e atualize a linha **Próxima tarefa**.
7. Após 2–3 tarefas, ou se a conversa passar de ~100k tokens: **encerre** e peça ao usuário para abrir uma sessão nova com o prompt de `docs/tasks/PROMPTS.md`.

## Regras de ouro
- **Não invente APIs, flags nem versões.** Se não tiver certeza: leia o código da dependência (`node_modules/<pkg>/*.d.ts`, `%USERPROFILE%\.cargo\registry\src\*\<crate>-*\src`), use `cargo doc`/`--help`, ou a documentação oficial (web), e confirme antes de usar.
- **Não decida arquitetura.** As decisões estão nos cartões e em `docs/CHEATSHEET.md`. Se faltar decisão, pergunte.
- **Não mude o escopo do cartão.** Nada de "aproveitar para refatorar". Se achar um problema fora do escopo, anote em `PROGRESS.md` (*Observações*).
- **Não avance** com testes falhando, lint quebrado ou `TODO` escondido. Teste desabilitado só com comentário + motivo.
- **Segredos:** nunca logar token, chave ou ticket inteiros (no máximo 4 caracteres). Comparar segredos em tempo constante.
- **Texto de UI:** nunca fixo no código; sempre chave de i18n (pt-BR e en). Rust/Kotlin enviam **códigos**, não frases.
- **Kernel/driver:** nunca instalar driver experimental na máquina principal (só VM/PC de teste).
- **Windows/PowerShell:** os comandos dos cartões são PowerShell, a partir da raiz do repositório. Caminhos com espaço sempre entre aspas.

## Higiene de contexto (importante)
- Mostre só o **fim** de saídas longas: `comando 2>&1 | Select-Object -Last 40`.
- Não cole arquivos grandes na conversa; use `Select-String`/`rg` para achar o trecho.
- Não releia `docs/protocol.md` inteiro: use `docs/CHEATSHEET.md` (resumo) ou o trecho que o cartão cita.
- Um arquivo de código novo deve ter **no máximo ~300 linhas**; divida em módulos.

## Pistas rápidas
- Resumo de nomes, portas, tipos de mensagem e limites: `docs/CHEATSHEET.md`.
- Como os cartões funcionam e prompts prontos: `docs/tasks/README.md` e `docs/tasks/PROMPTS.md`.
- Visão geral (só se o usuário pedir): `LocalCamera-PLAN.md`. Detalhes de protocolo: `docs/protocol.md`. UI: `docs/ui-spec.md`, `docs/ui/*.md`. Testes: `docs/testing.md`.
