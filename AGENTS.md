# AGENTS.md — LocalCamera

Projeto: celular Android vira webcam e microfone para Windows 11 (Wi-Fi ou USB). Monorepo pnpm + Rust + Kotlin + C++.
**Você tem pouco contexto. Trabalhe em passos pequenos e não abra documentos à toa.**

## Fluxo obrigatório (siga SEMPRE, uma tarefa por vez)
1. Abra `PROGRESS.md`. Pegue a **primeira tarefa `[ ]`** cuja dependência já está `[x]`.
2. Abra **somente o cartão dela**: `docs/tasks/fase-N/<ID>.md`. Ele traz tudo o que você precisa. Só abra outro arquivo se o cartão mandar.
3. Os rótulos `humano` e `agente+humano` são históricos e **não** autorizam handoff manual: a IA deve executar os testes por ADB e pelos harnesses permitidos. Se não houver um caminho automatizado fiel e seguro, registre o critério como bloqueado/PARCIAL/PENDENTE e peça apenas uma decisão técnica ou de escopo — nunca peça ao usuário para operar o aparelho/PC ou coletar evidência.
4. Execute os passos na ordem. Depois de cada bloco, rode o comando de **verificação** do cartão.
5. Falhou 2 vezes no mesmo ponto? **Pare**, escreva em `PROGRESS.md` (seção *Bloqueios*) o erro e o que tentou, e pergunte ao usuário; não repita o mesmo comando sem mudar a causa.
6. Cumpriu o **Critério de pronto**? Faça o commit (mensagem sugerida no cartão), marque `[x]` em `PROGRESS.md` e atualize a linha **Próxima tarefa**.
7. **Não encerre por uma contagem fixa de tarefas.** Continue até concluir a fase/objetivo solicitado, respeitando dependências, critérios de pronto e commits seletivos. Abra sessão nova quando o contexto se aproximar de ~100k tokens, perder confiabilidade, ou quando um bloqueio exigir decisão humana. Se o usuário autorizar workers, delegue análises independentes a workers Luna; não permita edições concorrentes na mesma tarefa/arquivos, não pule a sequência do `PROGRESS.md` e revise, integre e verifique todo resultado antes de marcá-lo pronto.

## Velocidade e controle de execução
- Antes de testar, confira se já existe evidência aprovada que cobre exatamente o critério e a versão em teste. **Reutilize-a**; não repita soak, coleta ou teste só para mudar intervalo/formato de saída. Repetição longa exige critério novo não coberto ou autorização explícita.
- Execute uma vez cada verificação prescrita pelo cartão. Depois de uma mudança localizada, prefira o teste direcionado; não repita a suíte inteira se nada nela mudou. Não reexecute tarefas já verdes para “completar” um relatório.
- Para comandos longos, informe a duração esperada e deixe o progresso visível. Não combine execução em background com `Select-Object -Last` (isso retém a saída até o fim). Se usar background, grave saída incremental e continue apenas com trabalho independente; não deixe o usuário esperando sem atualização.
- Se não houver progresso observável por 10 minutos, inspecione a saída e o processo uma única vez. Se estiver travado, encerre somente o processo iniciado por esta tarefa, registre como **interrompido/incompleto** (nunca verde) e avance com o que não depende dele. Não fique consultando repetidamente o mesmo processo.
- Soaks e testes com duração inerente acima de 15 minutos só podem ser iniciados quando o cartão exigir e não existir evidência aprovada equivalente; informe a duração antes. Não interrompa nem repita um teste obrigatório silenciosamente.
- Mantenha a resposta focada em resultado, bloqueio e próxima ação concreta; não narre leituras/comandos rotineiros.

## Mandato de testes: IA + ADB (obrigatório)
- Todo teste de aparelho/UI deve ser operado pela IA via ADB (`shell input`, instrumentação, `uiautomator`, `screencap`, `logcat`, `dumpsys`) e harness local. Não delegar passos, confirmações, screenshots, ações físicas ou inspeção visual ao usuário.
- Use apenas mecanismos documentados e disponíveis no dispositivo. Uma configuração/injeção via ADB só substitui a ação original se cobrir o mesmo critério; simulação não equivalente deve ser identificada e não pode virar aprovação.
- Quando o critério exigir desktop, sensor/ação física, outra rede, chamada ou julgamento visual, procure primeiro uma automação segura controlável pela IA. Se não existir, mantenha o item sem aprovação, registre precisamente o bloqueio e solicite uma decisão técnica/aceitação de desvio — nunca um teste manual.
- Inspecione screenshots capturadas por ADB quando isso resolver o critério, mas não arquive imagens com conteúdo de outros apps/dados pessoais; interrompa a interação se houver risco de atuar sobre conteúdo alheio e apague a captura temporária.
- Os campos Executor `humano`/`agente+humano` em cartões antigos não se sobrepõem a este mandato. Não altere evidência histórica; replaneje apenas a tarefa ativa.

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
