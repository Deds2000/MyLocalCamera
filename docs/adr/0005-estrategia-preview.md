# ADR 0005 — Estratégia de preview: JPEG do Rust (modo B)
- **Data:** 2026-10-29
- **Status:** aceito

## Contexto
A UI do desktop precisa exibir o preview da câmera em tempo real. O spike S5 (`docs/spikes/S5.md`) comparou duas estratégias com protótipos medidos (i7-10700F + RTX 3090, WebView2 155.0.4283.62, escala 100%, ~8,5 min de dados por modo, fixture 1080p em loop a 30 fps):

- **A:** Rust encaminha o H.264 (Annex-B, AUs) e o WebView2 decodifica com **WebCodecs**.
- **B:** Rust entrega **JPEGs reduzidos 960×540** do quadro já composto; o frontend decodifica com `createImageBitmap` e desenha no canvas.

Critérios do cartão: ≥30 fps estáveis; sem crescimento de memória em 10 min; **preferir B** se a diferença de CPU for pequena (o preview mostra exatamente a saída da câmera virtual, com uma só decodificação).

| Métrica | A (WebCodecs) | B (JPEG 960×540) |
|---|---|---|
| fps / drops | 29,99 / 0 | 29,77 / 0 |
| MB/s no Channel | 1,133 | 0,653 |
| CPU main / árvore | 28,3% / 54,0% | 16,7% / 48,2% |
| Memória (início→fim) | 29→55 MB | 29→37 MB |
| Latência p50/p95 (ms) | 12 / 20 | 10 / 14 |

Custo da decodificação H.264 do lado Rust (medido por velocidade, `spikes/s5/measure-decode.ps1`): ≈0 com `d3d11va`; ≈4,4% de 1 núcleo em software multi-core; ≈14,6% em software 1 thread.

## Decisão
Adotar **B**: o Rust decodifica o fluxo (ou recebe o quadro já decodificado), reduz para 960×540, codifica JPEG (q~70) e envia pelo `ipc::Channel` a 30 fps; o frontend usa `createImageBitmap` + canvas, fechando cada bitmap. O preview reflete exatamente o que a câmera virtual entrega (uma única decodificação, do lado Rust).

## Consequências
- Menos banda no Channel (−42%, 0,65 × 1,13 MB/s) — importante em Wi-Fi.
- Menos CPU (árvore: 48,2% × 54,0%; processo main: 16,7% × 28,3%) e menos memória (37 × 55 MB); latência menor (p95 14 × 20 ms).
- O caminho crítico Rust ganha decode + downscale + encode JPEG. O decode sai ≈0 com `d3d11va` (hardware) e é modesto em software; o downscale + encode JPEG ainda não foi medido — validar nas próximas fases (esperado: poucos % de 1 núcleo).
- Trade-off aceito: em DPI 150/200% o preview de 960×540 é esticado pelo compositor e fica menos nítido que o decode nativo 1080p (modo A). Grupos 150/200% não foram medidos por decisão do usuário (2026-10-29); se o produto exigir nitidez em alta DPI, reabrir a medição (execuções já automatizadas em `spikes/s5/run-s5.ps1`).
- WebCodecs continua viável como alternativa de upgrade para preview 1080p: o modo A funcionou (30 fps, 0 drops, memória flat) e o suporte H.264 foi confirmado neste runtime.

## Alternativas consideradas
- **A (WebCodecs + H.264 no WebView2):** funcional, mas mais CPU, mais banda e decodificação delegada ao WebView (depende de driver GPU; a spec WebCodecs **não obriga** suporte a H.264 — a checagem `isConfigSupported` é obrigatória no produto). Descartado porque a diferença de CPU foi pequena (≈0,06 núcleo na árvore) e B leva as vantagens do critério de desempate (banda, latência, saída idêntica à câmera virtual).
- **Medição em 150/200% de DPI:** pulada por decisão do usuário (mudar a escala neste PC exige sign-out); o veredito A/B foi decidido com os dados de 100%.
