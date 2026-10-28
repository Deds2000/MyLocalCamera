# ADR 0004 — Backend de câmera do Android (CameraX primário, Camera2 no modo dual)
- **Data:** 2026-10-28
- **Status:** aceito
## Contexto
O app precisa capturar a câmera do Android e alimentar o `MediaCodec` (H.264, 1080p30, Surface) de
forma estável, com controle de fps e suporte a câmeras simultâneas (modo dual). O spike S3 mediu
CameraX 1.6.2 × Camera2 num Samsung SM-G781B (Android 13, SM8250), 10 min por cenário.

## Decisão
- **CameraX** é o backend de **câmera única** (`Preview` + `SurfaceProvider` entregando a Surface do encoder).
- **Camera2** (sessões concorrentes) é o backend do **modo dual** (traseira + frontal/ultrawide, dois encoders).
- A interface `CameraBackend` (Fase 2) expõe dois modos: `single` (CameraX) e `dual` (Camera2 concorrente);
  o encoder (`MediaCodec` em Surface, 1080p30, keyframe 1/s) é o mesmo nos dois.
- Obrigatório no CameraX: `ResolutionSelector` com aspecto 16:9 + 1920x1080 (senão cai em 1088x1088).
- Controle de fps via `Camera2Interop` (`CONTROL_AE_TARGET_FPS_RANGE`); no modo dual, faixa do aparelho.

## Consequências
- **Muda na Fase 2:** `CameraBackend` com modo `single`/`dual`; `CapabilitiesProbe` reporta
  `maxFps = 30` e `concurrentModes` (grupos + resolução máx por stream).
- **Muda na Fase 7 (dual):** usar Camera2 concorrente como referência; resolver o grupo concorrente por
  `availableConcurrentCameraInfos`/`getConcurrentCameraIds` e negociar resolução por stream.
- **Limita as promessas:** `capabilities.maxFps = 30` (o aparelho não oferece 60); sem `KEY_LOW_LATENCY`
  nesse encoder → latência dominada pelo GOP (1º keyframe ~0,7–1,1 s), não contar com flag de baixa latência.
- **Muda no app real:** streaming deve rodar em **serviço em primeiro plano** (sobrevive à tela apagada);
  deixar só na Activity para o teste na tela.

## Alternativas consideradas
- **Só CameraX (também para dual):** descartado como caminho comprovado — o dual foi validado com sessões
  concorrentes do Camera2; CameraX expõe o suporte, mas o pipeline dual via CameraX ainda não foi testado.
- **Só Camera2 (também para câmera única):** viável (T2: 29,7 fps estável) e unificaria o backend, mas o
  CameraX dá o ciclo de vida/lifecycle e a seleção de resolução de forma mais simples para o modo único,
  que é o caso padrão. Mantido como fallback se o CameraX trouxer instabilidade em outros aparelhos.
