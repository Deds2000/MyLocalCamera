# Matriz de aparelhos — LocalCamera

Primeira versão (F2-T15, 2026-11-11). Fontes: `docs/spikes/S3.md` (limites reais),
`docs/tests/fase-2.md` + `PROGRESS.md` (resultados Fase 2) e os CSVs em
`docs/tests/evidence/fase-2/`. Valores de CSV em cultura invariável.

## Samsung SM-G781B (Galaxy S20 FE)

**Serial ADB:** `RXCT800A2HR` · **Android:** 13 (API 33) · **SoC:** Qualcomm SM8250 (Snapdragon 865)

### Capacidades (probe F2-T06 + Spike S3)

| Item | Valor |
|---|---|
| Encoder H.264 | `OMX.qcom.video.encoder.avc` (nenhum `c2.*`) |
| Perfis/níveis | Constrained Baseline, Baseline, Main, Constrained High, High — todos no nível `L131072` |
| `KEY_LOW_LATENCY` | não suportado — latência só via GOP (1º keyframe ~0,7–1,1 s) |
| Faixas de fps (traseira) | 15-15; 7-24; 24-24; 7-30; 30-30 — **teto 30 fps, sem 60** |
| 1º keyframe (S3) | 701–1158 ms |
| rotationHint | 90° retrato / 0° paisagem (sensor 90°) |
| Câmeras concorrentes | `back+front` (cam0+cam1) e `back+ultrawide` (cam0+cam3), cada stream 1080p30 |
| Veredito S3 | CameraX (single) + Camera2 (dual); `ResolutionSelector` 16:9 obrigatório (senão cai em 1088x1088) |

### Desempenho (Fase 2)

| Item | Resultado | Referência |
|---|---|---|
| IT-2.05 (1080p30) | 1.034 quadros; pico 30,5 fps / 8.565 kbps | `fase-2.md` |
| PT-2.01 (fps 10 min) | 29,78 fps média (min 28,0) em 660 s — ≥ 29,5 com 11/61 amostras pontuais abaixo | `fase-2.md` |
| PT-2.02 (latência p95) | 86,8–92,4 ms (> 80 ms) — **desvio aceito pelo usuário** | `fase-2.md` |
| PT-2.03 (CPU/temp 10 min) | 675 s reais, 46 amostras: média 29,78 fps (min 28,0, max 30,1); 19.697 quadros; bateria 35,9→38,2 °C; status térmico 1→2; PSS 149.658–172.931 KiB; CPU média 33,7% (máx 36,1%); sem throttling visível — **OK** | `PROGRESS.md` (2026-11-11); `evidence/fase-2/pt-203-samsung-thermal-20261111.csv` |
| PT-2.04 (bateria) | removido do escopo por decisão do usuário (2026-11-11) | `fase-2.md` |
| PT-2.05 (partida a frio) | 612–665 ms (≤ 1,5 s) — OK | `fase-2.md` |

### Longa duração

| Item | Resultado | Evidência |
|---|---|---|
| LT-2.01 (30 min) | rerun 2026-11-11: 120 amostras × 15 s; PID constante (sem crash/restart); PSS 145.916–173.655 KiB (média 158.053; delta início→fim −8.997 KiB); temperatura 39,0→44,2 °C; servidor: 178 intervalos, média 29,75 fps (min 27,9, max 32), 52.908 quadros; **sem desconexão** — **OK** | `evidence/fase-2/lt-201-samsung-soak-20261111.csv` |
| LT-2.02 (50 ciclos) | 50 ciclos conectar/parar; zero crash; PSS inicial/final 34.999/64.153 KiB, delta +29.154 KiB (limite +40.000) — **OK** | `evidence/fase-2/lt-202-samsung-20261111.csv` |

> O coletor padrão `scripts/soak-f2.ps1` (1 s; colunas `t,fps,kbps,mem_mb,temp_c`) já existe e está
> commitado (`ed07d5e`); bugs de leitura incremental/PSS e codificação no Windows PowerShell 5.1
> foram corrigidos e o piloto ADB (15 s) passou com FPS, PSS e temperatura em todas as amostras.
> Os números de LT-2.01 da tabela acima vêm do rerun da F2-T14 (amostragem a 15 s / do servidor),
> cuja evidência está aprovada. A primeira execução canônica de 30 min com `soak-f2.ps1` foi
> interrompida em t=1027 s: ADB perdeu o dispositivo, o coletor encerrou e o servidor registrou
> `ConnectionLost`. As 1.025 amostras completas foram preservadas em
> `evidence/fase-2/lt-201-samsung-soak-20261111-partial.csv`, mas **não** comprovam 30 min
> contínuos. Após o coletor ser corrigido para tolerar erro transitório do ADB, houve uma segunda
> tentativa: o pareamento seguro e o vídeo foram confirmados; por decisão do usuário, a coleta extra
> foi cancelada após 423 s e seu CSV parcial (424 linhas) está em
> `evidence/fase-2/lt-201-samsung-soak-20261111-attempt2-cancelled.csv`. O usuário dispensou a
> coleta adicional de 1 s/`soak-f2.csv` como redundante; a evidência aprovada da F2-T14
> (120 × 15 s, citada na tabela) permanece a fonte oficial para LT-2.01. Os harnesses, o estado de
> pareamento e os arquivos temporários foram limpos após o cancelamento; `pcs: []`, secrets vazio,
> porta 48651 livre.

### Quirks e comportamento registrado

- **Não reconecta sozinho após queda de rede** (suspeita de defeito, FT-2.01): entra em
  `Reconectando…` e, com o Wi-Fi totalmente restaurado, nenhuma tentativa por > 4 min.
  Reconexão manual (Meus PCs → Conectar) funciona. Hipótese: laço de retry preso ao
  `NetworkHandle` antigo de `WifiNetwork.acquire()`.
- Botão `Parar` **não responde** durante `Reconectando…` (force-stop encerra).
- Mensagem de câmera ocupada é genérica (“Falha na comunicação…”, FT-2.05) — não específica de câmera.
- Banner `mobile.permissions.denied` na Home dispara por mic/notificações ≠ `granted` (UX, não câmera).
- A revogação de CAMERA por `pm` faz o **AMS matar o processo** (`permissions revoked`); o app
  encerra a sessão limpo antes disso (`camera_disabled` → `PeerClosed`).
- Android 13 ignora o broadcast `AIRPLANE_MODE`; usar `cmd connectivity airplane-mode enable/disable`.
- Rotação física: UI manteve retrato com stream ativo; `STREAM_CONFIG 1920x1080@30 rot=90 face=back`
  confirmado em sessão separada (AT-2.14, parcial).

## Moto G (5)

**Serial ADB:** `0030261879` · **Android:** 8.1 (API 27) · **SoC:** não registrado · **Encoder:** `OMX.qcom.video.encoder.avc` (perfil `unspecified`, nível `n/a`, F2-T08) — fonte: `PROGRESS.md`. O probe detalhado (S3/F2-T06) rodou apenas no Samsung; o `CapabilitiesProbeTest` (IT-2.04) passou neste aparelho.

### Resultados (sessões 2026-11-03 e anteriores)

| Item | Resultado |
|---|---|
| IT-2.05 (720p30) | `STREAM_CONFIG 1280x720@30`; 10 amostras estáveis, média **30,8 fps** (min 29,7, max 32,1); encerramento pelo app (`PeerClosed`) |
| IT-2.02 (bitrate) | 117.661 bps / 291 frames em 10 s (CBR não anunciado pelo encoder hardware — registrado como diagnóstico) |
| IT-2.04 (probe) | `CapabilitiesProbeTest` (2 testes) coerente com o dispositivo |
| IT-2.06 (persistência/reconexão) | OK — pareamento salvo/listado, reconexão por `pcId` sem novo pareamento, esquecimento |
| ST-2.02 (segredos em log) | 812 linhas capturadas sem token/URI/Base64/padrões hex |
| ST-2.04 (deep link malicioso) | 3 URIs inválidas (host inválido, porta 99999, nome de 10 KB) — app permaneceu vivo, sem sessão |
| CameraBackendSmokeTest | 1 teste, 11,8 s |

### Alertas e pendências

- **Alerta (médio):** ao parar o stream, `CameraXBackend: unbind: Not in application's main thread`
  antes de `parado` e encoder liberado — investigar o caminho de stop (não é crash).
- Leitura QR **não comprovada** no Moto (permissão e preview ao vivo confirmados).
- LT-2.01/2.02 e PT-2.0x **não executados** neste aparelho — pendência do relatório da fase.
- QR do Moto fica para o final por decisão do usuário.
