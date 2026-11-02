# Replanejamento — fechamento da Fase 2 e preparação do M1

**Data:** 2026-11-02 · **Escopo:** planejamento documental, sem alteração de arquitetura ou dispensa de testes.

## 1. Base e limites da revisão

Base: `PROGRESS.md`, `LocalCamera-PLAN.md`, cartões F1-T17/F2-T14/F2-T15/F2-GATE e especificações das Fases 2/3. Os resultados abaixo são **registros anteriores**, não testes reexecutados nesta revisão. A situação dos aparelhos e de processos descritos em 2026-10-31 precisa ser reconfirmada; “suíte rodando” não é resultado.

- **Concluído:** Fases 0 e 1 aprovadas; servidor independente do Tauri; protocolo e vetores; mocks, fixtures e CI.
- **Implementado com validação parcial:** telas mobile, plugin Kotlin, captura/encoder, conexão, fila prioritária, armazenamento, serviço e seleção de rede.
- **Últimos resultados registrados:** JS mobile 31/31, shared 13/13, JVM 87/87 e catálogo e2e 12/12. Não substituem os testes instrumentados, manuais ou de longa duração.
- **Disponível no disco:** `scripts/soak-f2.ps1` e `crates/lc-server/examples/soak_server_lan.rs`. Reutilizar e conferir, não recriar pelo texto antigo do F2-T15.
- **Ainda ausente nesta revisão:** `docs/devices.md` e `docs/tests/fase-2.md`; existência de coletor não comprova execução de soak.
- **Não entregue:** preview desktop integrado, webcam virtual de produção, USB/mDNS, controle completo, dual e microfone virtual.

## 2. Decisões consolidadas

| Decisão já sustentada | Consequência para o planejamento |
|---|---|
| S3: CameraX única e Camera2 dual | Não repetir spike nem antecipar dual; validar cada aparelho. 60 fps não é promessa de entrega |
| S2: pipe nomeado, ADR 0003 | Fase 4 segue esse canal; não reabrir comparação com memória compartilhada sem nova evidência |
| S5: estratégia B, ADR 0005 | Gerar somente a trilha JPEG/Rust na Fase 3; usar o mesmo quadro composto da futura webcam |
| Fases 0/1 aprovadas com desvios registrados | Preservar histórico; auditorias não executadas e DPI 150/200% não viram “aprovados” |

O PC continua servidor; o celular continua dono do estado. Sem mudança de protocolo, dependências ou driver nesta revisão.

## 3. Caminho crítico da Fase 2

Ordem de retomada proposta **dentro dos cartões existentes**. Não autoriza pular dependências ou iniciar tarefas futuras.

| Bloco | Responsabilidade | Saída necessária |
|---|---|---|
| 1. Recuperar contexto físico | Agente + humano | Confirmar aparelhos autorizados/conectados e localizar resultados/logs da última suíte; registrar o que terminou, falhou ou não foi executado |
| 2. Isolar bloqueio do encoder | Agente + humano, F2-T08/T14 | Samsung reiniciado com autorização, execução instrumentada identificada por aparelho/build e logcat preservado. Moto G é resultado separado, não substituto silencioso |
| 3. Fechar validações de origem | Agente + humano, T05/T06/T07/T08/T12/T13/T16 | Evidências de armazenamento/backup, probe, câmera, encoder, notificação, plugin e rede; fechar cada cartão somente após seu critério |
| 4. Integração LAN | Agente + humano, F2-T14 | IT-2.05/2.06: parear, transmitir e reconectar com chave; registrar aprovação, encoder, resolução e fps reais |
| 5. Aceitação, falhas e segurança | Agente + humano, F2-T14 | AT/FT/PT/ST do cartão com evidência individual; resolver lacuna de QR e limitações do método de backup |
| 6. Sustentação | Agente + humano, F2-T15, após T14 | LT-2.01 de 30 min, LT-2.02 de 50 ciclos, PT-2.03/2.04 e matriz de aparelhos; CSV invariável e logs |
| 7. Portão | Agente, F2-GATE | CI/vetores/regressão aplicável, relatório e decisão explícita sobre desvios; só então solicitar cartões da Fase 3 |

**Parada do diagnóstico:** há três falhas já registradas no Samsung. Não abrir nova rodada de tentativas sem combinar a retomada com o usuário. Após reinício, se o problema persistir, coletar a evidência e propor um cartão corretivo delimitado; watchdog ou troca de implementação não estão aprovados por este documento.

**Aparelhos:** o critério do portão exige ao menos um aparelho; o F2-T15 descreve Samsung e recomenda segundo alvo. Usar apenas Moto G, mudar o aparelho principal ou reduzir cobertura exige decisão registrada, não inferência de que ambos se comportam igual.

## 4. Lacunas a resolver antes de fechar o portão

| Lacuna | Encaminhamento | Quem decide |
|---|---|---|
| Scanner QR registrado como stub no F2-T03; objetivo/AT exigem QR | Conferir estado atual. Se persistir, criar cartão corretivo com biblioteca/API verificadas, permissões, cleanup e testes do fluxo real. Alternativa: desvio explícito aprovado, sem apresentar manual/deep link como leitura de QR | Usuário aprova cartão ou desvio |
| Dependências A+H continuam `[~]`, mas T14 já iniciou sua validação agregada | Tratar os resultados do T14 como evidência dos cartões de origem; fechar estes antes de concluir T14. Se o fluxo exigir dependência por “parte A pronta”, formalizar nos cartões antes de mudar a regra de elegibilidade | Usuário aprova eventual mudança de fluxo |
| F2-GATE cita RS-S com desktop/`vcam-probe`, ainda futuros | Mapear a parcela executável agora; identificar a parcela dependente das Fases 3/4 e pedir adiamento explícito para F4-GATE. CI do servidor não comprova webcam virtual | Usuário aprova eventual adiamento |
| `adb backup` pode não fornecer extração útil no Android alvo | Confirmar suporte e método no aparelho. Backup vazio/indisponível não prova proteção; combinar regras de exclusão e inspeção adequada, documentando alcance e limitações | Agente propõe método; usuário aceita eventual desvio |
| Playwright já capturou outro aplicativo na porta 5173 | Antes de novos baselines, verificar identidade do app e servidor dedicado. Planejar correção delimitada de porta/reuso para execução local/CI | Agente em cartão corretivo, sem alterar o Docker do usuário |
| Descrições antigas divergem do trabalho recente | No próximo cartão, corrigir `createInputSurface` descrito como anterior ao `configure`, hipótese “deve rodar verde” e infraestrutura ainda descrita como inexistente; não apagar o histórico | Agente responsável pelo cartão |

Estas lacunas **não são dispensas aprovadas**. Se não houver decisão, permanecem pendentes/bloqueadas.

## 5. Registro de evidência e aproveitamento dos ensaios

No F2-T14, manter uma linha por ID; F2-T15 complementa PT/LT; F2-GATE consolida o relatório. Evitar uma segunda matriz paralela neste documento.

Campos mínimos: **ID · aparelho/API · build/commit · data · método · resultado · caminho da evidência · limitação/desvio e aprovação**. Separar “não executado”, “bloqueado”, “falhou” e “aprovado”; compilação de androidTest não é aprovação de IT.

- Aproveitar a sessão de 10 min do AT-2.05 para PT-2.01/2.03/2.04, preservando condições, horários e valores de início/fim. Se faltar coleta, a métrica permanece pendente.
- Executar LT-2.01 e LT-2.02 após T14, conforme a dependência atual; a coleta de 10 min não substitui os ensaios longos.
- Para bateria, registrar se o aparelho estava carregando por USB; não inferir consumo válido de uma carga que permaneceu constante enquanto carregava.
- Registrar condições de rede/carga do PC, perfis/encoder, aquecimento e cadência real. Não transpor números do spike ou do Samsung para o Moto G.
- Logs, URI e capturas não devem expor token/chave/ticket completos; usar redação de no máximo quatro caracteres.
- Não matar ffmpeg ou interferir em outros apps. Reboots, mudanças de rede e ações manuais devem ser combinados com o usuário.

## 6. Preparação da Fase 3 sem antecipar implementação

Após F2-GATE aprovado, gerar cartões pequenos usando a especificação da Fase 3. Proposta de agrupamento para reduzir risco; dependências finais serão explicitadas nos cartões:

1. **Base de integração:** shell, i18n, comandos/eventos Tauri e store; integrar `lc-server`, sem copiar rede/confiança para uma segunda implementação.
2. **Fluxo real de conexão:** estado vazio/URI, aprovação e dispositivos confiáveis; demonstrar celular → servidor → estado refletido no desktop.
3. **Pipeline mínimo de vídeo:** decoder com fixture e fallback software, composição única NV12 e preview JPEG. Demonstrar fixture → quadro composto → preview, depois validar com celular.
4. **Robustez e desempenho:** recuperação de SPS/IDR, filas limitadas, bitmap liberado, latência por etapa, pausa de preview oculto e ausência de renderizações React por quadro.
5. **Interface restante e portão:** painel/chips/saúde/catálogo, acessibilidade, regressões, evidências de DPI e relatório.

Não trocar a ordem dos IDs em `PROGRESS.md` antes de gerar e conferir o DAG dos cartões. Não inserir câmera virtual, USB, dual ou áudio nesta fase.

## 7. Marcos e estimativas

- **Checkpoint F2:** celular transmite e reconecta de forma comprovada, com limitações declaradas. Ainda não é webcam utilizável nos apps Windows.
- **Checkpoint F3:** preview integrado e pipeline medido. Ainda não equivale ao M1.
- **M1 / F4-GATE:** fluxo completo até a câmera virtual, validado nos consumidores previstos, incluindo NVIDIA Broadcast; preservar política de isolamento para trabalho de driver.
- **M2–M5:** mantidos; S4 continua na Fase 8, sem habilitar modo de teste ou instalar driver experimental na máquina principal.

As estimativas originais por cartão não são previsão atual de calendário. Reestimar após destravar encoder/QR e, em cada nova fase, separar implementação, validação automatizada, sessões humanas e ensaios longos. Não prometer data ou percentual global com base apenas na contagem de tarefas.

## 8. Critério de conclusão desta revisão documental

- Plano mestre alinhado aos vereditos e ao progresso registrado.
- Próximas ações vinculadas aos cartões existentes, sem marcar testes como executados.
- Lacunas e decisões pendentes explícitas; nenhuma arquitetura, dependência ou dispensa alterada implicitamente.
- Verificar links locais adicionados, `git diff --check` e diff final. Não é necessário executar suíte do aplicativo para uma alteração somente documental.
