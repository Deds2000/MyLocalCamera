# LocalCamera — Estratégia de testes

Documento transversal: cada fase tem seu **plano de testes** em `docs/phases/`, e todos seguem as convenções abaixo.

## 1. Princípios

1. **Automatizar o que dá; medir o que não dá.** Lógica pura (protocolo, estado, composição, filas, algoritmos) é 100% testada em automático. Hardware (câmera, codec, GPU, Frame Server, driver, Wi-Fi) é testado em **aparelhos reais**, com procedimento escrito e evidência guardada.
2. **Determinismo.** Todo teste de rede usa o `mock-sender` (celular simulado) e relógios controlados (`tokio::time::pause`, relógio injetável no Kotlin). Nada de `sleep` arbitrário.
3. **Teste negativo vale tanto quanto o positivo.** Para cada caminho feliz, há pelo menos um caso de falha (entrada inválida, desconexão, tempo esgotado, recurso ausente).
4. **Cada bug corrigido ganha um teste** que o reproduz (regressão) antes da correção.
5. **Evidência.** Todo teste manual registra data, build, aparelho, resultado e, quando útil, captura de tela/log em `docs/tests/fase-N.md`.
6. **Sem testes "pulados" em silêncio.** Teste desabilitado exige comentário com motivo e ADR; teste instável (*flaky*) é corrigido ou colocado em quarentena com prazo.

## 2. Tipos de teste e identificadores

| Prefixo | Tipo | Onde roda | Exemplo |
|---|---|---|---|
| **UT** | Unitário (lógica isolada) | CI local | `UT-1.07` |
| **IT** | Integração (várias partes reais, sem hardware, usando `mock-sender`/probes) | CI local ou máquina de dev | `IT-4.03` |
| **AT** | Aceitação em aparelho real e apps reais (manual guiado ou semiautomático) | Bancada | `AT-4.01` |
| **FT** | Falha e caos (desconexões, entradas inválidas, recursos indisponíveis) | CI local (se simulável) ou bancada | `FT-2.05` |
| **PT** | Desempenho e latência (com metas numéricas) | Bancada | `PT-3.02` |
| **ST** | Segurança | CI local e bancada | `ST-1.04` |
| **LT** | Longa duração (*soak*) | Bancada | `LT-4.01` |
| **RT** | Regressão (lista do que reexecutar) | Conforme a fase | `RT-5` |

Coluna **Auto** nas tabelas: ✅ automático no CI · 🔧 automático na máquina de dev (precisa de GPU/Windows/Android) · ✋ manual guiado.

## 3. Ferramentas por camada

| Camada | Ferramentas |
|---|---|
| **Rust** | `cargo test`; `tokio::test` com `time::pause()`; `proptest` (propriedades do parser e do reducer); `cargo clippy -D warnings`; `cargo fmt --check`; opcional `cargo fuzz` no parser; `cargo deny`/`cargo audit` (licenças e vulnerabilidades) |
| **TypeScript/React** | Vitest; React Testing Library + `user-event`; verificação de acessibilidade automática com axe (ex.: `vitest-axe`; **confirmar**); ESLint (`i18next/no-literal-string`); `tsc --noEmit` |
| **Visual/UI** | Página `?catalog` de cada app + **Playwright** com capturas de referência (*snapshots*) rodando no navegador (sem Tauri/Capacitor) — **confirmar a configuração**; revisão manual com o checklist de `docs/ui-spec.md` §8 |
| **Kotlin/Android** | JUnit + `kotlinx-coroutines-test` (relógio virtual) para lógica pura; **`androidTest`** em aparelho real para MediaCodec/câmera/serviço; `lint` do Android; checagem de manifesto |
| **C++ (DLL e driver)** | Harness próprio com GoogleTest ou Catch2 para a lógica pura (cliente do pipe, placeholder, buffer circular, conversões); extrair a lógica do driver para uma biblioteca **testável em modo usuário** |
| **Ponta a ponta no desktop** | Opcional: WebDriver do Tauri (`tauri-driver`; **confirmar**) para fluxos críticos (aprovação, regenerar QR) |
| **Rede** | `mock-sender` (falhas determinísticas); para degradação de rede real: roteador com QoS, ou ferramenta de simulação no Windows (ex.: *clumsy*; **confirmar licença e uso**) |

## 4. Ferramentas de apoio do repositório (entram nas fases indicadas)

### 4.1 `tools/mock-sender` (Fase 1; ampliado nas seguintes)

| Flag | Efeito | Fase |
|---|---|---|
| `--uri`, `--host/--port/--token` | Pareia (usa o token do QR) | 1 |
| `--device-id`, `--name`, `--device-key` | Reconecta com chave | 1 |
| `--file arquivo.h264`, `--loop`, `--fps 30` | Reproduz vídeo por access unit com *pacing* | 1 |
| `--audio`, `--tone 440` | Envia PCM (seno) pelo socket de áudio | 1 |
| `--fake-version M.m` | Anuncia outra versão | 1 |
| `--fragment N` | Escreve em pedaços de N bytes (testa leitura fragmentada) | 1 |
| `--bad-magic`, `--huge-length`, `--bad-json`, `--unknown-type` | Quadros inválidos | 1 |
| `--delay-hello MS`, `--drop-ping`, `--no-pong` | Temporização e keepalive | 1 |
| `--slow KBPS` | Limita a vazão de envio | 1 |
| `--stall MS@SEG` | Para de enviar por MS ms depois de SEG s | 1 |
| `--kill-after SEG` | Fecha o socket de forma abrupta | 1 |
| `--reconnect N` | Conecta/desconecta N vezes | 1 |
| `--loss P`, `--corrupt P` | Descarta/corrompe P% dos access units | 3 |
| `--jitter MS` | Atraso aleatório entre envios | 3 |
| `--rotation-hint DEG`, `--size WxH` | `STREAM_CONFIG` customizado | 3 |
| `--emulate-phone` | Implementa o reducer de estado do celular (`SET`→`STATE`) | 6 |
| `--dual` | Dois streams (arquivos diferentes) | 7 |
| `--chaos SEED` | Sequência aleatória reproduzível de eventos | 6 |
| `--av-sync` | Flash no vídeo e bipe no áudio com o mesmo `ptsUs` | 8 |

### 4.2 Sondas (probes)
- **`tools/vcam-probe`** (Fase 4): abre a câmera virtual **como um app qualquer** (leitor de fontes do Media Foundation), lê N quadros e valida **resolução, formato, fps, jitter entre quadros, contador de quadros (ordem/perdas), cores por quadrante (orientação/espelho/NV12) e detecção de placeholder**. Saída em JSON.
- **`tools/audio-probe`** (Fase 8): captura do endpoint "LocalCamera Microphone" (WASAPI) e valida **frequência do tom (FFT), relação sinal/ruído, descontinuidades (estalos), duração e contagem de amostras versus relógio**.
- **`scripts/soak.ps1`**: amostra a cada 10 s CPU, memória (working set), *handles*, *threads* e GPU dos processos do app; grava CSV e aplica os critérios de §7.
- **`scripts/make-fixtures.ps1`**: gera os arquivos de teste (§5).

### 4.3 Padrão de teste de vídeo (`lc-pattern`)
Vídeo sintético gerado com `ffmpeg`: **4 quadrantes de cores fixas** (vermelho, verde, azul, amarelo, na ordem superior-esquerdo, superior-direito, inferior-esquerdo, inferior-direito) e uma **faixa superior com o contador de quadros em 16 bits** (blocos preto/branco via `drawbox` com `enable` dependente de `n`; **confirmar a sintaxe**). Permite ao `vcam-probe` provar que: a orientação está certa, não há espelhamento acidental, o NV12 está correto, e não há quadros perdidos, repetidos ou fora de ordem.

## 5. Fixtures (geradas por `scripts/make-fixtures.ps1`, fora do Git)

| Arquivo | Conteúdo | Uso |
|---|---|---|
| `fixtures/hd30.h264` | 1080p30, 10 s, `testsrc2` | Fases 1–3 |
| `fixtures/pattern1080.h264` | `lc-pattern` 1080p30, 20 s | Fases 3–7 |
| `fixtures/pattern720.h264` | `lc-pattern` 720p30 | Modo dual, rebaixamento |
| `fixtures/pattern43.h264` | `lc-pattern` 4:3 (1440×1080) | Letterbox/`cover` |
| `fixtures/sps-change.h264` | 720p → 1080p no meio (novo SPS/PPS) | Troca de resolução |
| `fixtures/corrupt.h264` | Um access unit corrompido (gerado por `tools/fixture-tool corrupt`) | Recuperação do decodificador |
| `fixtures/no-idr-start.h264` | Sem o primeiro AU (gerado por `fixture-tool drop-first-au`); começa em quadro P | Decodificador deve esperar keyframe |
| `fixtures/tone440.pcm` | Seno 440 Hz, 48 kHz, 16 bits, mono, 30 s | Fase 8 |
| `fixtures/avsync.*` | Flash + bipe sincronizados | Fase 8 |

## 6. Ambientes e matriz de dispositivos

Registrar em `docs/devices.md` (modelo, Android, SoC, encoders H.264, suporte a câmera simultânea, resultado dos testes).

| Item | Mínimo recomendado |
|---|---|
| Celulares | **2 aparelhos** de fabricantes diferentes (um deles com câmeras simultâneas, se possível) |
| PCs | **1 com GPU NVIDIA** (para o NVIDIA Broadcast); idealmente **1 sem** (testa decodificação por software) |
| Windows | Windows 11 em duas versões/builds recentes, **VM/PC de teste** para o driver |
| Rede | Wi-Fi 5 GHz e 2,4 GHz; um roteador com **isolamento de clientes** ligável; PC por cabo e por Wi-Fi |
| Cabos | Um cabo USB bom (dados) e um ruim/curto de carga (para casos de falha) |
| Ambientes | Com VPN ativa no PC e no celular; com adaptadores virtuais (Hyper-V/WSL); DPI do Windows em 100%, 150% e 200% |

## 7. Metas de desempenho (valores iniciais; ajustar com medições reais e registrar em ADR)

| Métrica | Meta inicial | Fase |
|---|---|---|
| Latência ponta a ponta (vidro a vidro) | ≤ 150 ms USB · ≤ 250 ms Wi-Fi 5 GHz | 3, 4 |
| Latência interna do PC (recepção → quadro entregue à câmera virtual), p95 | ≤ 30 ms | 4 |
| fps sustentado em 1080p30 | ≥ 29,5 | 2, 3, 4 |
| Variação do intervalo entre quadros da câmera virtual (desvio-padrão) | ≤ 3 ms | 4 |
| CPU do desktop (decodificação por GPU + composição + preview) com 1080p30 | ≤ 15% de um PC de 6 núcleos | 3 |
| Renderizações do React durante o stream | ≤ 4/s (só estatísticas) | 3 |
| Crescimento de memória | ≤ 20 MB/h no desktop · estável no celular | LT |
| *Handles* do processo | Estáveis (± 5%) em 60 min | LT |
| Troca de câmera | ≤ 1,5 s (p95) | 6 |
| Mudança de resolução/fps | ≤ 2 s (p95) | 6 |
| Convergência do bitrate adaptativo após limitar a banda | ≤ 5 s | 6 |
| Conexão por USB (clique → vídeo) | ≤ 3 s | 5 |
| Reconexão por mDNS | ≤ 4 s | 5 |
| Latência do áudio (microfone do celular → app no PC, Wi-Fi) | ≤ 150 ms | 8 |
| Erro de sincronia A/V (com ajuste) | ≤ 40 ms | 8 |
| Partida a frio do desktop / do app Android | ≤ 2 s / ≤ 1,5 s | 9 |

## 8. Critérios de aprovação para testes longos (LT)

Usar `scripts/soak.ps1`. **Aprovado** se, na duração indicada: sem crash nem desconexão não recuperada; fps médio dentro da meta; crescimento de memória e *handles* dentro dos limites do §7; nenhum erro novo nos logs; temperatura do celular registrada (sem desligamento por superaquecimento).

## 9. Suítes de regressão

| Suíte | Conteúdo | Quando |
|---|---|---|
| **RS-S (smoke, ~15 min)** | `pnpm ci`; mock → desktop → `vcam-probe` (1 min); um fluxo real: QR → aprovação → vídeo → Zoom/Meet vendo a câmera; USB (a partir da Fase 5) | No fim de **cada fase** |
| **RS-F (completa, ~1 dia)** | Todos os UT/IT; todos os AT das fases concluídas; FT principais; PT e um LT de 30 min; matriz de apps | Antes de cada **tag de marco** |
| **RS-L (longa)** | LT de 8 h (webcam) e de 2 h (áudio) | Antes de `v1.0.0` |

## 10. Relatório de testes

Para cada fase, `docs/tests/fase-N.md` (copiar `docs/tests/_template.md`):
- **Cabeçalho:** data, *commit*, versão do desktop e do APK, PC (CPU/GPU/Windows), celular (modelo/Android), rede.
- **Tabela de resultados:** ID · resultado (aprovado/reprovado/não aplicável) · medição · evidência (arquivo/captura) · observações.
- **Defeitos encontrados** (com ID, severidade, situação) e **desvios aceitos** (com ADR).
- **Veredito:** a fase só fecha com **todos os AT/FT/PT/ST obrigatórios aprovados** ou desvio aceito por escrito.

## 11. Dados de teste e privacidade
Não commitar vídeos com pessoas, identificadores reais de aparelhos nem tokens/chaves. Fixtures são sintéticas. Logs de exemplo devem ser anonimizados.
