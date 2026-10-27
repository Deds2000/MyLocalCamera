# Prompts prontos para o opencode (copie e cole)

## 1. Iniciar uma sessão (o mais usado)
```
Leia AGENTS.md e PROGRESS.md. Execute a próxima tarefa seguindo o fluxo do AGENTS.md.
Abra somente o cartão dela. Rode a verificação do cartão antes de marcar como pronta.
Ao terminar, faça o commit, atualize o PROGRESS.md e me diga em 3 linhas o que foi feito.
```

## 2. Continuar depois de uma interrupção
```
Leia AGENTS.md e PROGRESS.md. Veja `git status` e `git log -5 --oneline`.
Descubra em que ponto a tarefa atual parou, sem refazer o que já está commitado.
Abra o cartão da tarefa e continue do passo em que parou.
```

## 3. Depois de uma falha repetida
```
Pare. Não tente de novo. Resuma em até 10 linhas: o comando que falha, a mensagem de erro
(só as últimas 30 linhas), o que já tentou e sua hipótese. Registre isso em PROGRESS.md na
seção "Bloqueios" e aguarde minha orientação.
```

## 4. Revisar o que foi feito antes de fechar a tarefa
```
Revise sua própria tarefa contra o "Critério de pronto" do cartão, item por item.
Para cada item, mostre o comando ou arquivo que comprova. Liste qualquer coisa fora do escopo
que você alterou (deve ser nada). Rode `pnpm run ci` (ou `scripts/ci.ps1 -Fast`) e mostre só o resumo final.
```

## 5. Contexto cheio (antes de encerrar a sessão)
```
Estamos perto do limite de contexto. Atualize o PROGRESS.md (status, "Próxima tarefa",
"Observações") com tudo o que um colega precisaria para continuar, faça commit do que estiver
verde, e pare. Não inicie outra tarefa.
```

## 6. Quando a próxima tarefa estiver marcada "(gerar)"
Não é para o agente. Diga ao autor do plano: "Gere os cartões da Fase N", informando os vereditos dos spikes em `docs/spikes/`.

## 7. Quando a tarefa for de Executor `humano`
```
A próxima tarefa é de Executor "humano". Mostre-me o roteiro do cartão em passos curtos,
diga exatamente que resultado devo colar de volta, e aguarde. Não marque como pronta.
```
