# ADR 0001 — Decisões fixas do projeto
- **Data:** 2026-10-27
- **Status:** aceito
## Contexto
(o problema e as restrições)
## Decisão
- Só Windows 11.
- Tauri 2.
- Um celular por vez.
- Dois sockets TCP na mesma porta (principal: controle + vídeo; áudio).
- Pareamento por QR + aprovação no PC + chave por dispositivo.
- mDNS para reconexão.
- Sem criptografia do tráfego (uso próprio).
- 1080p30 padrão.
- Celular é dono do estado ("último comando vence").
- Espelho e rotação aplicados no PC.
- Câmera dupla só se o aparelho suportar.
- Modo de teste do Windows aceito para o driver (ou driver de terceiros assinado, se o spike S4 permitir).
## Consequências
Mudar qualquer item exige um novo ADR que substitua este.
## Alternativas consideradas
(e por que foram descartadas)
