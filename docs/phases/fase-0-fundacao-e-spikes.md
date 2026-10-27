# Fase 0 — Fundação, infraestrutura de testes e spikes de risco

**Marco:** M1 (`v0.1.0`) · **Tamanho:** M (≈ 5–8 dias) · **Depende de:** nada
**Relatório:** `docs/tests/fase-0.md`

> **Execução:** para implementar, **não** use este arquivo diretamente. Siga `PROGRESS.md` e os cartões em `docs/tasks/fase-0/`. Este arquivo é a especificação e o plano de testes da fase.

## 1. Objetivo e resultado observável
Ter o monorepo, as ferramentas, o CI local e a infraestrutura de testes funcionando, e **eliminar as quatro maiores incertezas técnicas** antes do código de produto: câmera virtual × NVIDIA Broadcast (**S1**), canal Frame Server ↔ app (**S2**), backend de câmera do Android (**S3**) e estratégia de preview (**S5**). O spike do driver de áudio (S4) fica na Fase 8, de propósito.

**Resultado observável:** `pnpm ci` verde; a janela Tauri e o APK abrem mostrando um componente shadcn com os tokens do preset; quatro relatórios de spike com veredito e ADR.

## 2. Leitura obrigatória
`LocalCamera-PLAN.md` (§3, §5, §8), `docs/testing.md` (inteiro), `docs/ui-spec.md` §1.

## 3. Escopo
**Dentro:** esqueletos que compilam e rodam; CI local e hook; infraestrutura de testes (Vitest, Rust, Kotlin, fixtures, relatório); spikes S1, S2, S3, S5 com relatório e decisão.
**Fora:** qualquer funcionalidade de produto (rede, UI final, câmera virtual real).

## 4. Entradas e saídas
| Entradas | Saídas |
|---|---|
| Plano, protocolo (rascunho), specs de UI | Repositório com esqueletos; `scripts/ci.ps1`; `.githooks/`; `scripts/make-fixtures.ps1`; `docs/adr/0000–000x`; `docs/spikes/S1,S2,S3,S5.md`; `docs/tests/fase-0.md`; protótipos descartáveis em `spikes/` (fora do build de produção) |

## 5. Plano de trabalho

### Etapa A — Repositório e ferramentas
- [ ] **F0-T01 — Monorepo pnpm**
  - Passos: `pnpm-workspace.yaml` (`desktop`, `mobile`, `packages/*`); `package.json` raiz com `packageManager` fixando a versão do pnpm e `engines` do Node; `.nvmrc`; `tsconfig.base.json` com `strict: true`; `.editorconfig`; `.gitattributes` (`* text=auto eol=lf` exceto `.ps1`/`.bat` em CRLF se necessário); `.gitignore` (fixtures, `target/`, `node_modules/`, `dist/`, `android/build`); `LICENSE` (MIT); `README.md` mínimo.
  - ESLint (config plana, TypeScript, React, hooks, **`i18next/no-literal-string`** em `desktop/src` e `mobile/src`), Prettier.
  - Pronto quando: `pnpm install --frozen-lockfile && pnpm lint && pnpm typecheck` passa numa pasta limpa.
- [ ] **F0-T02 — `desktop/`**
  - Passos: criar o app Tauri 2 com React 19 + TS + Vite; Tailwind 3.4 (`tailwind.config.ts` usa o preset compartilhado); instalar framer-motion, lucide-react, clsx, tailwind-merge; **configurar o shadcn/ui para Tailwind 3.4 + React 19** (confirmar a versão da CLI na documentação; se necessário, copiar componentes à mão); `components.json`; identificador `com.localcamera.desktop`; janela sem decoração (**confirmar** o comportamento no Windows 11); **CSP restritiva** no `tauri.conf.json` e capacidades (`capabilities/`) com o mínimo necessário.
  - Pronto quando: `pnpm dev:desktop` abre a janela com `Button`, `Tabs` e `Dialog` estilizados pelos tokens.
- [ ] **F0-T03 — `mobile/`**
  - Passos: projeto Capacitor 8.5 (`appId: com.localcamera.app`) + `@capacitor/app` + React 19 + Vite + Tailwind 3.4 + shadcn/ui; **sem** framer-motion; `android/` gerado; `android:allowBackup="false"` já no manifesto; WebView *debuggable* só em debug.
  - Pronto quando: o APK de debug instala no celular real e abre com um componente shadcn estilizado.
- [ ] **F0-T04 — `packages/shared`**
  - Passos: `cn()`; `tailwind-preset.ts` com os tokens de `docs/ui-spec.md` §1.2–§1.3 (cores, raios, sombras `shadow-glass`/`shadow-overlay`/`glow-accent`, fontes); variáveis CSS do shadcn mapeadas; esqueleto de i18n (`pt-BR`, `en`) com um teste que falha se uma chave existir em um idioma e não no outro; pasta `protocol/` vazia.
  - Pronto quando: os dois apps importam `@localcamera/shared` e usam o preset.
- [ ] **F0-T05 — Workspace Rust**
  - Passos: `Cargo.toml` raiz com `crates/lc-protocol`, `tools/mock-sender`, `desktop/src-tauri`; `rust-toolchain.toml` fixo; `rustfmt.toml`; `clippy` com `-D warnings`; `deny.toml` (licenças permitidas) para `cargo deny` opcional.
  - Pronto quando: `cargo build --workspace && cargo test --workspace` passa.
- [ ] **F0-T07 — Documentação base**
  - Copiar `AGENTS.md` e `LocalCamera-PLAN.md` para a raiz; criar `docs/adr/0000-template.md`, `docs/spikes/`, `docs/tests/`; criar `docs/perf/`.
- [ ] **F0-T08 — ADR 0001 (decisões fixas)**
  - Registrar Tauri 2, Windows 11, dois sockets, mDNS no MVP, estado dono no celular, modo de teste aceito.

### Etapa B — CI local
- [ ] **F0-T06 — `scripts/ci.ps1` e hook**
  - Passos: parâmetros `-Fast` (lint + typecheck + testes unitários rápidos), `-SkipAndroid`, `-Only rust|ts|android`; cada etapa mede o tempo e o resumo final lista **etapa × tempo × resultado**; código de saída ≠ 0 ao primeiro erro (exceto com `-KeepGoing`); etapas: `pnpm install --frozen-lockfile` → `lint` → `typecheck` → `test` → `cargo fmt --check` → `cargo clippy --workspace -- -D warnings` → `cargo test --workspace` → (`./gradlew testDebugUnitTest lintDebug` em `mobile/android`).
  - `.githooks/pre-push` → `scripts/ci.ps1 -Fast`; `pnpm prepare` configura `core.hooksPath`.
  - Pronto quando: um erro proposital em **cada camada** (TS, lint, Rust clippy, teste Kotlin) faz o CI falhar (ver FT-0.01 a FT-0.04).

### Etapa C — Infraestrutura de testes
- [ ] **F0-T09 — Vitest + RTL** nos dois apps e no `shared` (ambiente `jsdom`, `setupTests`, utilitário `renderWithProviders` com i18n e store). Teste de exemplo por pacote.
- [ ] **F0-T10 — Playwright para o catálogo** (opcional agora, obrigatório na Fase 2/3): configuração para abrir `?catalog` de cada app no navegador e comparar capturas; pasta `docs/ui/screens/`. **Confirmar** a configuração do Playwright com Vite.
- [ ] **F0-T11 — Kotlin:** dependências de JUnit e `kotlinx-coroutines-test`; módulo/pastas `src/test` e `src/androidTest`; um teste de exemplo de cada tipo (o `androidTest` roda no aparelho).
- [ ] **F0-T12 — Rust:** `proptest` e `tokio` (com `test-util`) nas dependências de teste; helper de teste para portas livres.
- [ ] **F0-T13 — `scripts/make-fixtures.ps1`:** gera os arquivos de `docs/testing.md` §5 com `ffmpeg` (`-bf 0`, `-tune zerolatency`, `-x264-params "aud=1:keyint=30:repeat-headers=1"`, `-f h264`). Os arquivos **não** vão para o Git; o script é idempotente e imprime o hash de cada um.
- [ ] **F0-T14 — Template de relatório** (`docs/tests/_template.md`, já fornecido) e o script `scripts/new-report.ps1 <fase>` que copia o template preenchendo data, commit e versões.

### Etapa D — Spikes (protótipos descartáveis em `spikes/`)
Cada spike termina com `docs/spikes/Sx.md` (**pergunta, método, dados medidos, veredito, riscos restantes**) e um ADR.

#### S1 — Câmera virtual × NVIDIA Broadcast
**Pergunta:** a câmera virtual por Media Foundation aparece e funciona como **entrada do NVIDIA Broadcast** e nos demais apps?
**Passos**
1. Obter o exemplo oficial de câmera virtual da Microsoft (repositório `microsoft/Windows-Camera`, `Samples/VirtualCamera`; **confirmar o caminho atual**) e compilar com Visual Studio 2022.
2. Instalar a DLL em `C:\Program Files\LocalCameraSpike\` (legível pelo Frame Server), registrar o CLSID com `regsvr32` (administrador) e criar a câmera com `MFCreateVirtualCamera` + `Start` a partir de um app de console. **Anotar** os parâmetros usados (tempo de vida, acesso, categorias) e **se a DLL precisa ser assinada** (testar sem e com assinatura de teste).
3. Para **cada app**, executar o procedimento da tabela e preencher o resultado: Câmera do Windows; Configurações → Câmeras; Chrome (Google Meet); OBS; Zoom; Teams; Discord; **NVIDIA Broadcast** (lista de entrada, e depois aplicando um efeito).
4. No Broadcast: testar (a) abrir o app com a câmera já ativa, (b) ativar a câmera com o app já aberto, (c) reiniciar o Broadcast, (d) trocar de câmera de entrada.
5. Se o Broadcast **não** listar a câmera: repetir com um filtro **DirectShow** (Softcam ou equivalente) e com os dois registrados ao mesmo tempo.
6. Testar o comportamento com **dois apps** usando a câmera ao mesmo tempo e com o app Câmera do Windows ocupando-a.
**Métricas:** listada (S/N), imagem exibida (S/N), fps percebido, atraso até aparecer, erros no log do app/Visualizador de Eventos.
**Veredito:** `mf-ok` · `mf+directshow` · `somente-directshow` · `inviável` (este último exige parar e discutir).

#### S2 — Canal Frame Server ↔ app
**Pergunta:** como entregar frames à DLL (no Frame Server) **sem rodar o app como administrador**?
**Passos**
1. **Named pipe** criado pelo app (usuário comum) com DACL que concede acesso à conta do Frame Server (**confirmar qual conta/SID o serviço usa**) e ao usuário atual; rejeitar clientes remotos; a DLL do S1 conecta como cliente.
2. **Memória compartilhada** em `Global\` criada **sem elevação**: verificar se é possível (privilégio necessário) e se a DLL consegue abrir.
3. Medir 1080p30 NV12 (3.110.400 bytes/quadro ≈ 93 MB/s): vazão máxima, CPU de cada lado, quadros perdidos, latência de escrita→leitura (p50/p95).
4. Verificar **detecção de cliente** (o app sabe quando a DLL conecta/desconecta?) e o comportamento ao fechar/abrir a câmera 20 vezes.
5. Matar o app com a câmera aberta e o inverso (reiniciar o Frame Server com o app ativo): a DLL e o app se recuperam?
**Veredito:** `pipe` · `shm` · `outro (descrever)`.

#### S3 — Backend de câmera do Android: CameraX × Camera2
**Pergunta:** o CameraX atende o MVP ou precisamos de Camera2 (total ou parcial)?
**Passos** (app Kotlin descartável, no celular real)
1. **CameraX:** `Preview` entregando a `Surface` de entrada de um `MediaCodec` H.264 (CBR, 8 Mbps, keyframe 1 s, baixa latência) — 1080p30 por 10 min.
2. **Camera2:** o mesmo.
3. **1080p60:** o aparelho oferece? (faixas de fps em `CameraCharacteristics`); no CameraX, testar `Camera2Interop` para fixar a faixa de fps.
4. **Câmeras simultâneas:** API de câmeras simultâneas do CameraX **e** `getConcurrentCameraIds()` do Camera2; combinações, **resolução máxima por stream** e abrir as duas codificando por 5 min.
5. **Orientação:** obter o `rotationHint` correto em retrato/paisagem (CameraX: `TransformationInfo`; Camera2: orientação do sensor).
6. **Encoders:** nome do encoder (`c2.*`/`OMX.*`), suporte a `KEY_LOW_LATENCY`, perfis/níveis, modo CBR, tempo para o primeiro keyframe.
7. Interrupções: chamada recebida, outro app abrindo a câmera, tela apagando com *foreground service*.
**Métricas:** fps real, jitter, latência câmera→buffer codificado, temperatura após 10 min, uso de CPU, falhas.
**Regra de decisão:** CameraX se cumprir **tudo** que o MVP exige; onde falhar, Camera2 **só nessa parte**. O código usa a interface `CameraBackend`.

#### S5 — Estratégia de preview no desktop
**Pergunta:** A) WebCodecs no WebView2, ou B) preview reduzido gerado pelo Rust?
**Passos**
1. **A:** `VideoDecoder.isConfigSupported({ codec:'avc1.640028', avc:{ format:'annexb' }, hardwareAcceleration:'prefer-hardware', optimizeForLatency:true })` (a especificação **não obriga** o suporte a H.264); enviar `fixtures/hd30.h264` por `tauri::ipc::Channel` binário; renderizar em `<canvas>`.
2. **B:** Rust decodifica (Media Foundation), reduz para ~960×540, codifica JPEG (qualidade ~70) a 30 fps e envia por Channel; frontend usa `createImageBitmap` → canvas.
3. Em ambos: fps estável, quadros descartados, vazão do Channel (MB/s), CPU/GPU, latência, memória em 10 min, comportamento ao minimizar, ao pausar o stream e com **DPI 100/150/200%**.
4. Medir também com `fixtures/corrupt.h264` e `sps-change.h264`.
**Critérios:** ≥ 30 fps estáveis em 1080p; sem crescimento de memória; **preferir B** se a diferença de CPU for pequena (mostra exatamente a saída da câmera virtual, uma decodificação só).

## 6. Armadilhas conhecidas
- Registrar a DLL em pasta do usuário faz o Frame Server não conseguir carregá-la (outra conta).
- Nem toda GPU/driver oferece decodificação H.264 em hardware no WebView2; sempre checar `isConfigSupported`.
- `adb`, `ffmpeg` e caminhos com espaços: sempre usar aspas nos scripts PowerShell.
- Em CI local, não dependa de GPU, Android ou Frame Server: esses testes são `🔧`/`✋`.

## 7. Plano de testes

### 7.1 Unitários / integração
| ID | Tipo | Cenário | Resultado esperado | Auto |
|---|---|---|---|---|
| UT-0.01 | UT | `cn()` com classes conflitantes do Tailwind | Mantém a última (via tailwind-merge) | ✅ |
| UT-0.02 | UT | Preset exporta todas as cores/raios/sombras de `ui-spec` §1.2–§1.3 | Chaves e valores iguais à especificação | ✅ |
| UT-0.03 | UT | Paridade de i18n: toda chave de `pt-BR` existe em `en` e vice-versa | Teste passa; remover uma chave o faz falhar | ✅ |
| UT-0.04 | UT | ESLint `no-literal-string` em um componente com texto fixo | Lint falha | ✅ |
| UT-0.05 | UT | `make-fixtures.ps1` executado duas vezes | Mesmos hashes; sem erro | 🔧 |
| IT-0.01 | IT | `pnpm ci` completo em pasta limpa | Verde; resumo com tempos | 🔧 |
| IT-0.02 | IT | `pnpm ci -Fast` | Termina em ≤ 3 min | 🔧 |
| IT-0.03 | IT | Build do desktop (`tauri build --debug`) e do APK debug | Ambos geram artefatos | 🔧 |

### 7.2 Falha (testes "do próprio CI")
| ID | Cenário | Resultado esperado |
|---|---|---|
| FT-0.01 | Introduzir erro de tipo em TS | `typecheck` falha e o CI para |
| FT-0.02 | Introduzir aviso do `clippy` | CI falha |
| FT-0.03 | Introduzir teste Kotlin que falha | CI falha |
| FT-0.04 | `git push` com lint quebrado | Hook de `pre-push` bloqueia |
| FT-0.05 | Rodar o CI sem `ffmpeg`/sem Android SDK | Mensagem clara dizendo o que falta (`-SkipAndroid` funciona) |

### 7.3 Aceitação
| ID | Passos | Resultado esperado | Auto |
|---|---|---|---|
| AT-0.01 | `pnpm install` e `pnpm ci` numa pasta limpa | Tudo verde | 🔧 |
| AT-0.02 | `git push` com um erro de lint proposital | O hook bloqueia | 🔧 |
| AT-0.03 | `pnpm dev:desktop` | Janela abre com componentes shadcn estilizados | ✋ |
| AT-0.04 | Instalar o APK debug no celular | App abre com componente shadcn | ✋ |
| AT-0.05 | Abrir `docs/spikes/` | S1, S2, S3 e S5 com veredito e ADR | ✋ |
| AT-0.06 | `desktop` em produção (`tauri build`) abre DevTools? | **Não** (DevTools desativadas em release) | ✋ |
| AT-0.07 | Inspecionar o manifesto Android | `allowBackup=false`, só `MainActivity` exportada, sem permissões extras | ✋ |

### 7.4 Spikes (cada um com sua tabela de resultados em `docs/spikes/`)
| ID | Verificação | Evidência exigida |
|---|---|---|
| PT-0.01 | S2: vazão e latência do canal escolhido a 1080p30 NV12 | Tabela p50/p95, CPU dos dois lados |
| PT-0.02 | S3: fps, jitter e latência de captura→codificação (CameraX × Camera2) | Tabela por backend |
| PT-0.03 | S5: CPU/GPU/latência/memória das estratégias A e B | Tabela e gráfico |
| ST-0.01 | S2: o canal escolhido **não** aceita cliente remoto nem outro usuário do Windows | Teste com segundo usuário/outra máquina |
| ST-0.02 | CSP e capacidades do Tauri: nenhum recurso remoto carregado | Inspeção das requisições |

### 7.5 Regressão
Esta fase é a base: guardar os resultados do **IT-0.01** como referência de tempo do CI.

## 8. Checklist de saída
- [ ] `pnpm ci` verde; hook funcionando; FT-0.01–FT-0.05 comprovados.
- [ ] Desktop e APK abrem com o design system.
- [ ] **S1, S2, S3 e S5** com veredito e ADR; se algum for *no-go* sem plano B, **parar e discutir com o usuário**.
- [ ] `docs/tests/fase-0.md` preenchido e aprovado.

## 9. Riscos da fase e contingência
| Risco | Contingência |
|---|---|
| Exemplo da Microsoft não compila/muda de caminho | Procurar a documentação atual de `MFCreateVirtualCamera`; construir a fonte de mídia mínima a partir da documentação |
| CLI do shadcn/ui incompatível com Tailwind 3.4 | Copiar os componentes manualmente da documentação legada |
| Aparelho sem câmeras simultâneas | Tratar a Fase 7 como "recurso condicional" e testar com `caps` simulados no `mock-sender` |
