# F2-FIX-04 — auditoria de lacunas (2026-11-16)

Escopo: classificação e disponibilidade de automação dos dez IDs. Esta auditoria
não aprova critérios nem substitui eventual decisão D6. Matriz e PROGRESS não
foram editados. Evidências anteriores só são reutilizadas quando cobrem o mesmo
critério; nenhuma delas cobre os critérios pendentes abaixo.

## Disponibilidade e limites

`adb devices -l` encontrou somente `0030261879`, Moto G (5), online. `pm list
packages` confirmou os pacotes próprios `com.localcamera.app` e
`com.localcamera.app.test`. Wi-Fi estava ligado e conectado; SSID omitido deste
registro. Não acionei tela/pareamento, não limpei dados, nem alterei rede,
permissões ou estado do aparelho. Este inventário não é evidência de aprovação
dos dez critérios. Evidência aprovada histórica do Samsung não equivale a uma
execução atual nos critérios em aberto.

## Critérios originais e mapa individual

As fontes primárias são `docs/phases/fase-2-mobile-transmissao-minima.md`,
§§7.3–7.5, linhas 96–141. `F2-T14.md` linhas 43–45 só diz para executar os
grupos via ADB/servidor e aponta para §§7.3/7.4; não redefine seus critérios.
`docs/testing.md` §7 contém metas gerais, não as definições destes dez IDs.

| ID | Critério auditado | Categoria / destino | Evidência reutilizável e cobertura | Resultado desta auditoria |
|---|---|---|---|---|
| AT-2.06 | “Desligar o Wi-Fi do PC por 5 s e religar”; esperado: “Celular reconecta sozinho com backoff” (`docs/phases/fase-2-mobile-transmissao-minima.md:104`) | 3; removido do escopo por decisão de 2026-11-11, conforme registro da linha AT-2.06 em `docs/tests/fase-2.md:61` | Decisão removeu o cenário; F2-FIX-01 é queda/reconexão de rede no celular e não cobre desligar Wi-Fi no PC | N/A preservado; sem nova execução |
| AT-2.07 | “Limitar a banda durante a transmissão”; esperado: quadros descartados, keyframe forçado, bitrate reduzido, `PING/PONG` continuam (fase-2-mobile-transmissao-minima.md:105) | 2 — envolve controle de banda + estado/vídeo no PC e protocolo; destinar ao gate F3/F4 quando integração desktop real puder observar tudo | Nenhuma evidência da matriz cobre limitação de banda. Não foi identificado harness de shaping/observação que valide os quatro resultados juntos; nenhuma ferramenta externa foi instalada | PENDENTE; destino gate F3/F4, critério completo acima |
| AT-2.08 | “Remover o celular da lista de confiáveis e reconectar”; esperado: receber `revoked` e apagar a chave (fase-2-mobile-transmissao-minima.md:106) | 2 — a remoção inicia na lista confiável do PC; cenário ponta a ponta requer UI desktop, chave persistida real no Android e reconexão Android | `DeepLinkDispatchTest.kt:140–177, 272–276` pareia por broker e limpa apenas `pcId` do teste, mas não revoga servidor. O runbook `F2-T14-harness-lan.md:8–15, 68–82, 89–95` descreve auto-approve/reconexão, sem UI/comando de revoke. `PrimaryConnectionTest.kt:149–170` simula resposta `revoked` e verifica key removida, porém `makeRig` usa store fake/peer local, não KeyStore Android. Resultado JUnit sanitizado: [`F2-FIX-04-ut209.json`](F2-FIX-04-ut209.json) | **PARCIAL técnico**, não resultado da matriz nem aprovação AT: a unit JVM passou no PC em 16 s (1 testcase; XML suite time 0,183 s); scope fake store/local peer. Falta revogação pela UI real do desktop e observação da chave no `AndroidKeyStore` do aparelho. Encaminhar a lacuna para avaliação no `F2-GATE-R` existente; proposta condicionada de futura integração desktop F3/F4, sem criar cartão/cobertura inexistente. Não limpar dados existentes |
| AT-2.09 | “Versão incompatível simulada”; esperado: texto “Atualize o app do PC” com as duas versões (`docs/phases/fase-2-mobile-transmissao-minima.md:107`) | 2 — precisa validar um peer/harness fiel que simule incompatibilidade de protocolo e a apresentação de ambas as versões no fluxo ponta a ponta; destino gate F3/F4 | `mobile/src/screens/DefinitiveErrorScreen.tsx:56–70` renderiza os valores `phone`/`pc` somente quando recebe `version_mismatch`; `HelloBuilder.kt:35` envia protocolo atual `1.0`; `SessionPhase.kt:257` reconhece reason terminal. No `it_lan_harness` não foi identificado nesta auditoria um modo de peer/protocolo incompatível (`it_lan_harness.rs:28, 59, 89–105` mostra opções de aprovação, sem essa opção). UI tests de mapeamento `connection.test.ts` não são negociação real | PENDENTE; não se afirma que a simulação seja impossível nem se propõe versão 2 em produção. Falta peer/harness de teste fiel à incompatibilidade simulada e assert da UI com ambas as versões; destino gate F3/F4 |
| AT-2.10 | “Escanear o QR com a câmera nativa do Android”; esperado: abre LocalCamera já com a URI (fase-2-mobile-transmissao-minima.md:108; F2-T01.md:50 especifica “QR real”) | 3 — sensor e QR apresentado externamente são parte explícita do critério | AT-2.13 testa formulário manual; `DeepLinkDispatchTest` entrega ACTION_VIEW direto via `onNewIntent`, sem câmera/decoder, e portanto não substitui QR nativo | PENDENTE; propor desvio explícito por QR/sensor físico ausente, não simular a câmera |
| FT-2.02 | “Mudar o celular de rede Wi-Fi”; esperado: host antigo falha e app pede novo QR (`docs/phases/fase-2-mobile-transmissao-minima.md:119`) | 3 — requer segunda rede acessível/controlável; uma segunda rede não foi demonstrada nesta auditoria | F2-FIX-01 valida queda/reconexão, não troca de rede nem host antigo após troca | PENDENTE; propor desvio/decisão para execução em ambiente com segunda rede isolada e controlável |
| FT-2.04 | “Receber uma chamada telefônica durante a transmissão”; esperado: pausa/encerramento limpo, aviso e app sem travar (fase-2-mobile-transmissao-minima.md:121) | 3 — exige chamada real; não fazer chamada nem tratar `am`/evento simulado como equivalente | FT-2.05 abre outro app de câmera; não cobre chamada nem estado de telefonia | PENDENTE; propor desvio explícito/decisão de escopo; nenhuma chamada executada |
| FT-2.09 | “Pouca memória (abrir apps pesados)”; esperado: sem crash e reconexão ao voltar (`docs/phases/fase-2-mobile-transmissao-minima.md:126`) | 3 — a origem da pressão é outro app pesado; ambiente isolado adequado não foi identificado nesta auditoria | LT-2.02 mede crescimento de PSS após ciclos, não pressão de memória. `am send-trim-memory` é só notificação ao app, não pressão real (critério distinto) | PENDENTE; propor desvio/decisão para ambiente isolado; não abrir apps alheios nem fingir pressure |
| FT-2.14 | “Escanear 20 QR codes inválidos seguidos”; esperado: sem crash/vazamento e faixa de erro some após 3 s (fase-2-mobile-transmissao-minima.md:131) | 3 — texto original exige escanear códigos com a câmera; teste do parser com strings/falsos quadros não prova câmera/UX da faixa | `AT-2.02` é QR válido pelo scanner in-app, não sequência de 20 inválidos; `DeepLinkDispatchTest` é ACTION_VIEW, não câmera. Não foi encontrado harness que produza uma sequência de QR físicos visíveis ao sensor | PENDENTE; propor desvio por dependência de cena/sensor; qualquer teste de parser separado não substituiria esse critério |
| PT-2.06 | “Tempo QR → ‘Transmitindo’ (com aprovação imediata)”; meta ≤4 s (fase-2-mobile-transmissao-minima.md:141; F2-T14.md:45) | 2 — o cronômetro inclui QR, aprovação e estado de transmissão; exige fluxo QR + aprovação desktop ponta a ponta | AT-2.13 evidenciou conexão manual e transmissão sem medir esse intervalo. Auto-approve `it_lan_harness` exclui UI de aprovação e não é equivalente (`F2-T14-harness-lan.md:12–15`) | PENDENTE; destino gate F3/F4 com QR/aprovação UI real e relógio monotônico; não atribuir tempo de subetapa |

## Execução

Executado somente o teste unitário direcionado:
`.\gradlew.bat :app:testDebugUnitTest --tests
com.localcamera.app.net.PrimaryConnectionTest.ut209_revokedApagaChaveENaoReconecta`
(cwd `mobile/android`): `BUILD SUCCESSFUL`, 1 task executada, 79 up-to-date,
16 s. O teste injeta resposta fake `revoked`, comprova um evento Denied, chave
ausente e nenhuma reconexão; não cobre UI desktop, rede real nem AndroidKeyStore.
Nenhum teste funcional novo foi executado. Outras execuções exigiriam QR/sensor
físico, chamada, segunda rede, pressão concorrente em apps alheios, ou integração
de desktop ainda não presente no harness. Nenhum PENDENTE foi promovido.

As lacunas de AT-2.08 e AT-2.09 não ficam como promessa indefinida: para AT-2.08
o unitário já executado cobre apenas o trecho sintético; a UI de revogação do PC
e o AndroidKeyStore precisam de integração real. Para AT-2.09 falta peer
negociando incompatibilidade e assert da UI com ambas as versões; nenhum
harness existente foi encontrado. Encaminhar ambas ao `F2-GATE-R` existente;
propor futura integração desktop F3/F4 somente se houver cobertura real. FT-2.09
segue sem execução porque não há ambiente isolado para gerar pressão de memória
sem afetar apps alheios; encaminhar ao `F2-GATE-R` para decisão de escopo e
condicionar eventual gate F3/F4 a ambiente controlado. Não existem, nem foram
criados aqui, cartões `F3-GATE` ou `F3-T14`.

As demais lacunas não N/A deste mapa também se encaminham ao `F2-GATE-R`
existente; itens que dependem de desktop só entram em proposta futura de gate
F3/F4 se houver ferramenta e cobertura demonstradas. Não se cria cobertura ou
cartão por encaminhamento.

Comandos de inventário: `adb devices -l`; `adb shell pm list packages` (filtrado
localmente para nomes do app próprio); `adb shell dumpsys wifi` (inspeção local,
SSID omitido). Sem logs brutos, screenshots ou segredos arquivados.
