# Fase 9 — Segundo plano, polimento e instalação (marco M5)

**Marco:** M5 → tag `v1.0.0` · **Tamanho:** M (≈ 8–12 dias) · **Depende de:** Fases 1–8
**Relatório:** `docs/tests/fase-9.md` · **Cartões:** (gerar) quando a Fase 8 fechar

## 1. Objetivo e resultado observável
Fechar a versão 1.0: comportamento em segundo plano configurável no celular, acabamento, i18n completo, acessibilidade, diagnóstico, empacotamento e documentação de instalação **reproduzível em outra máquina**.

**Resultado observável:** alguém instala do zero **só com o README** e usa webcam + microfone no dia a dia; a regressão completa (RS-F + RS-L) passa.

## 2. Leitura obrigatória
`docs/ui-spec.md` (§4–§8), `docs/ui/mobile.md` §3.11–§3.13, `docs/ui/desktop.md` §2.10, `docs/protocol.md` §2 e §12, `docs/testing.md` (§9 suítes).

## 3. Escopo
**Dentro:** configurações de segundo plano; ajuda de bateria; notificação; revogação de permissões; auditoria de i18n; compatibilidade de versões; estados de erro; acessibilidade; diagnóstico; revisão de desempenho; marca; empacotamento; documentação; instalação limpa; regressão final.
**Fora:** novos recursos (backlog).

## 4. Plano de trabalho (tarefas)
### Segundo plano (celular)
- [ ] **F9-T01** Configurações (Sheet) com **dois interruptores independentes**: *Continuar transmitindo com a tela apagada* (serviço em primeiro plano ativo ao sair do app) e *Manter a tela ligada* (`FLAG_KEEP_SCREEN_ON`). Ambos desligados → a transmissão **para** quando o app sai da frente (com aviso `mobile.settings.bothOff`).
- [ ] **F9-T02** Ajuda de bateria (`ui/mobile.md` §3.12) com botão que abre as configurações do sistema; texto neutro (sem citar marcas).
- [ ] **F9-T03** Notificação: estado (conectado/reconectando), ação **Parar**, textos em `strings.xml` (en e pt-rBR), canal `streaming` com importância baixa.
- [ ] **F9-T04** Reconexão em segundo plano com *backoff*; economia de energia (sem blur/animações durante a transmissão).
- [ ] **F9-T05** Revogação de permissões em tempo de execução (câmera/microfone retirados com o app aberto): parar com aviso, sem travar.
### Polimento
- [ ] **F9-T06** Auditoria de i18n: script que checa **paridade pt-BR/en**, chaves usadas × existentes, e **pseudo-localização** (+40% de texto, acentos) para achar layouts frágeis; nenhum texto fixo (lint verde); todos os códigos de erro traduzidos.
- [ ] **F9-T07** Compatibilidade de versões: telas e mensagens finais de `version_mismatch` (qual lado atualizar, com as duas versões); política documentada (major quebra, minor compatível); testes com `--fake-version`.
- [ ] **F9-T08** Estados de erro e vazios revisados (sem ADB, sem permissão, porta ocupada, firewall, câmera virtual não registrada, driver ausente, sem rede), cada um com ação de correção.
- [ ] **F9-T09** Acessibilidade: teclado completo no desktop, ARIA, foco visível, contraste AA, escala de fonte no mobile, `prefers-reduced-motion`, TalkBack/Narrador nos fluxos principais.
- [ ] **F9-T10** Diagnóstico: "Exportar diagnóstico" (versões, `caps` do aparelho, encoder, estatísticas recentes, estado dos indicadores) **sem** token/chave/ticket; níveis de log configuráveis.
- [ ] **F9-T11** Revisão de desempenho (`ui-spec` §4): stats a 2 Hz, preview fora do React, pausa ao minimizar; metas de CPU/GPU em `docs/perf/v1.md`.
- [ ] **F9-T12** Marca (`ui-spec` §7): ícone SVG (lente + arco, legível em 16 px), PNG/ICO/ICNS por script, ícone adaptativo e monocromático no Android, splash do Android 12+, "Sobre" com versões e licenças.
- [ ] **F9-T12a** Revisão final de UI: checklist de `ui-spec` §8 em **todas** as telas dos dois apps; capturas atualizadas em `docs/ui/screens/`; divergências intencionais em `docs/adr/`.
### Empacotamento e documentação
- [ ] **F9-T13** Desktop: `tauri build` (instalador NSIS) + `scripts/install.ps1` com `-Uninstall` (DLL da câmera virtual, regra de firewall, driver de áudio de teste).
- [ ] **F9-T14** Android: APK de release assinado com chave local; versão sincronizada com o desktop; instruções de `adb install`.
- [ ] **F9-T15** Documentação: `README` (requisitos, instalação, uso, solução de problemas), `docs/devices.md`, `docs/security.md` (modelo de ameaça: **sem criptografia**, token/chave, aprovação, o que **não** protege), changelog.
- [ ] **F9-T16** Instalação limpa em VM/PC novo seguindo **somente** o README.
- [ ] **F9-T17** Regressão final: RS-F completa + RS-L (8 h de webcam, 2 h de áudio).

## 5. Plano de testes
### 5.1 Unitários
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| UT-9.01 | Paridade e cobertura de i18n (script de auditoria) | Sem chaves faltando/sobrando | ✅ |
| UT-9.02 | Pseudo-localização (+40%) em todas as telas do catálogo | Sem corte/estouro (capturas) | 🔧 |
| UT-9.03 | Exportador de diagnóstico com segredos "plantados" (token, chave, ticket) | Nenhum aparece no arquivo | ✅ |
| UT-9.04 | Matriz de versões do protocolo e mensagens de "atualize" (qual lado) | Texto e lado corretos | ✅ |
| UT-9.05 | Lógica das duas opções de segundo plano (4 combinações) | Comportamento conforme `ui/mobile.md` §3.11 | ✅ |
| UT-9.06 | Migração de configurações (`schemaVersion` antigo/futuro) | Migra/ignora sem *crash* | ✅ |
| UT-9.07 | `install.ps1`: idempotência (duas execuções) e `-Uninstall` (simulado com diretório temporário) | Sem erro; estado final igual | 🔧 |
### 5.2 Integração
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| IT-9.01 | Instalação/desinstalação em VM limpa por script | Webcam/microfone funcionam; nada sobra após desinstalar | ✋ |
| IT-9.02 | Atualização de versão (v0.4 → v1.0) preservando pareamentos | Dispositivos confiáveis mantidos | ✋ |
| IT-9.03 | Downgrade (v1.0 → v0.4) | Falha de forma clara ou funciona; documentado | ✋ |
### 5.3 Aceitação
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-9.01 | Ativar "tela apagada", bloquear o celular por 10 min | Transmissão continua; notificação visível; câmera virtual com imagem | ✋ |
| AT-9.02 | Desligar os dois interruptores e sair do app | Transmissão para com aviso | ✋ |
| AT-9.03 | Revogar a permissão de câmera durante a transmissão | Aviso e parada limpa | ✋ |
| AT-9.04 | Trocar o idioma do sistema | Apps no idioma correto; sem texto fixo | ✋ |
| AT-9.05 | APK e desktop com majors diferentes | Mensagem clara dizendo **qual lado atualizar** | ✋ |
| AT-9.06 | Desktop só com teclado | Todas as ações acessíveis; foco visível | ✋ |
| AT-9.07 | Exportar diagnóstico | Arquivo sem segredos, com as informações listadas | ✋ |
| AT-9.08 | Instalação limpa em VM/PC novo pelo README | Webcam e microfone funcionam sem passos escondidos | ✋ |
| AT-9.09 | Desinstalar com `-Uninstall` | Câmera virtual, regra de firewall e driver removidos | ✋ |
| AT-9.10 | Regressão completa | Todos os AT-1.xx a AT-8.xx continuam passando | ✋ |
| AT-9.11 | Matriz de apps (câmera e microfone) | Todos verdes, **incluindo NVIDIA Broadcast** | ✋ |
| AT-9.12 | Revisão de UI completa (desktop e mobile, pt-BR e en, fonte a 200%, TalkBack, teclado) | Checklist de `ui-spec` §8 100% marcado | ✋ |
### 5.4 Falha e caos
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| FT-9.01 | Pouco espaço em disco na instalação | Mensagem clara; nada pela metade | ✋ |
| FT-9.02 | Instalar sem permissão de administrador | Pede elevação ou explica o que falta | ✋ |
| FT-9.03 | Desinstalar com o app aberto e com a câmera em uso | Fecha/avisa; sem tela azul nem resíduos | ✋ |
| FT-9.04 | Celular sem Wi-Fi e sem USB no primeiro uso | Estados e mensagens guiam o usuário | ✋ |
| FT-9.05 | Serviço do celular morto pelo sistema (economia de bateria) | Reconexão ou aviso claro | ✋ |
### 5.5 Desempenho
| ID | Cenário | Meta | Auto |
|---|---|---|---|
| PT-9.01 | Partida a frio do desktop / do app Android | ≤ 2 s / ≤ 1,5 s | ✋ |
| PT-9.02 | CPU do desktop ocioso (sem celular) | ≈ 0% | 🔧 |
| PT-9.03 | Bateria do celular em 30 min de 1080p30 (tela apagada) | Registrar (comparar com PT-2.04) | ✋ |
| PT-9.04 | Tamanho do instalador e do APK | Registrar | 🔧 |
### 5.6 Segurança
| ID | Cenário | Resultado esperado | Auto |
|---|---|---|---|
| ST-9.01 | `pnpm audit`, `cargo audit`, verificação de licenças | Sem vulnerabilidades conhecidas; licenças permitidas | 🔧 |
| ST-9.02 | Manifesto Android de release | Só `MainActivity` exportada; `allowBackup=false`; WebView sem depuração remota | ✋ |
| ST-9.03 | Desktop de release | DevTools desativadas; CSP aplicada; nenhuma requisição externa | ✋ |
| ST-9.04 | Revisão do modelo de ameaça (`docs/security.md`) | Honesto sobre "sem criptografia" e como mitigar | ✋ |
| ST-9.05 | APK de release assinado; checagem da assinatura | Válida | ✋ |
### 5.7 Longa duração
| ID | Cenário | Critério | Auto |
|---|---|---|---|
| LT-9.01 | **8 h** de webcam (RS-L) | Sem queda; memória/handles estáveis; temperatura registrada | ✋ |
| LT-9.02 | **2 h** de áudio contínuo | Sem estalos; deriva dentro do limite | ✋ |
| LT-9.03 | 24 h com o desktop aberto sem celular | CPU ≈ 0%; memória estável | 🔧 |
### 5.8 Regressão
**RS-F completa** + **RS-L** antes da tag `v1.0.0`.

## 6. Entrega do marco M5 (`v1.0.0`)
- [ ] Todos os testes obrigatórios aprovados; changelog; README validado em instalação limpa; `docs/tests/fase-9.md` aprovado; tag `v1.0.0`.

## 7. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| O sistema Android mata o serviço em alguns aparelhos | Ajuda de bateria + reconexão; documentar aparelhos problemáticos |
| Instalador/driver de teste confunde usuários | Guia ilustrado e scripts com diagnóstico passo a passo |
| Regressão encontra problemas tardios | Reservar ~30% do tempo da fase para correções |
