# Modelo de ameaças da LAN — LocalCamera

- **Data:** 2026-11-16
- **Código analisado:** commit `fef3301`
- **Escopo:** implementação desktop/Rust e protocolo neste commit. Este documento é análise estática; não houve teste dinâmico de rede, captura de tráfego, ataque MITM, fuzzing ou teste de resistência a DoS.

## Escopo e ativos

Ativos: vídeo H.264 e áudio PCM em trânsito; token de pareamento de 128 bits (32 dígitos hex) presente no QR/deep link; `deviceKey` de 256 bits (64 hex), entregue somente em aprovação “Permitir sempre”; trust store desktop (inclui token atual e hash SHA-256 das chaves); armazenamento privado do app Android que guarda `deviceKey`; decisão humana no diálogo de aprovação; PC/host e disponibilidade do serviço. A chave de dispositivo em claro não é persistida pelo servidor; a trust store inclui o token em claro (fonte: `docs/protocol.md:43-48,67-78`; `crates/lc-server/src/trust/store.rs:86-96,191-205`).

## Confiança e superfície

O desktop escuta por padrão em `0.0.0.0:48650`; quando a porta solicitada está ocupada, o servidor tenta porta alternativa (a porta 0 permite seleção de porta livre). Evidência: `desktop/src-tauri/src/server.rs:14,41`; `crates/lc-server/src/config.rs:8,38-42`; `crates/lc-server/src/lib.rs:181,268-271`. O protocolo usa TCP; `docs/protocol.md:10,24-35` declara explicitamente ausência de criptografia. Não há TLS/criptografia implementada no caminho de socket verificado (`crates/lc-server/src/lib.rs:268-271`, leitura/escrita de quadros em `crates/lc-server/src/handshake.rs:170`).

Um terceiro na mesma LAN que consiga observar o tráfego pode ler os quadros de vídeo/áudio e o HELLO/WELCOME, inclusive token durante pareamento e `deviceKey`/`audioTicket` quando transmitidos. Pode tentar autenticação, capturar e reutilizar credenciais, alterar/injetar quadros ou abrir conexões para consumir recursos. A conexão de controle/vídeo e a de áudio são TCP separados, mas isso não cifra nem autentica criptograficamente o host (`docs/protocol.md:24-35,74-78,91-119`).

O HELLO autentica por `pair` (token) ou `device` (`deviceId` + chave); pareamento novo passa por aprovação desktop por padrão e janela de 30 s (`docs/protocol.md:67-78`; `crates/lc-server/src/handshake/perform.rs:60-109`; `crates/lc-server/src/handshake/approval.rs:65-109`). Controles de pré-autenticação: até 8 conexões não autenticadas simultâneas e 5 s para o HELLO; 5 falhas de autenticação/IP em 60 s provocam bloqueio de 300 s; taxa de novas conexões é 20/IP em 10 s (`crates/lc-server/src/config.rs:10-24`; configuração desktop `desktop/src-tauri/src/server.rs:46-54`). O código não garante disponibilidade contra tráfego distribuído, múltiplos IPs ou saturação da rede/host.

## Ameaças concretas

| Ameaça | Impacto | Controle existente / ausente |
|---|---|---|
| Sniffing passivo | Exposição de vídeo, áudio, QR/token e credenciais de sessões capturadas. | **Ausente:** protocolo declara tráfego sem criptografia (`docs/protocol.md:10`). Token no QR (`docs/protocol.md:43-48`); áudio/vídeo em TCP (`docs/protocol.md:24-35,139-148`). |
| MITM/ARP spoofing e injeção/alteração | Interceptação, alteração, injeção de controle/mídia e possível personificação do host. | **Ausente:** sem TLS, identidade autenticada do PC, integridade criptográfica ou proteção contra MITM (`docs/protocol.md:10`; framing TCP `crates/lc-server/src/handshake.rs:170`). |
| Replay de `deviceKey`/token | Credencial capturada pode ser reutilizada enquanto válida para autenticação. | `deviceKey` validada contra hash; comparação em tempo constante, mas não é desafio-resposta; token é rotacionado após pareamento bem-sucedido e ao regenerar. Não há proteção contra replay de credenciais por captura no transporte não cifrado (`crates/lc-server/src/trust/store.rs:176-188,191-232`; `docs/protocol.md:74-78`). |
| Brute force e enumeração | Tentativas de adivinhar token/chave ou identificar estados/respostas de autenticação. | Limitadores: 5 falhas em 60 s/IP bloqueiam por 300 s e taxa 20 conexões/10 s/IP (`crates/lc-server/src/config.rs:16-24`; `crates/lc-server/src/trust/ratelimit.rs:30-105,121-171`). São por IP, não defesa global. Respostas distinguem `bad_token`, `not_approved`, `revoked`, `busy` etc. (`docs/protocol.md:119`); não são mensagens uniformes. |
| DoS por conexões/HELLO lentos | Ocupação de slots, CPU/recursos e indisponibilidade. | Limite de 8 não autenticadas, timeout HELLO de 5 s; taxa por IP. O semáforo e timeout aparecem em `crates/lc-server/src/lib.rs:223-234` e `crates/lc-server/src/conn/connection.rs:265-267`; valores em `crates/lc-server/src/config.rs:10-12,22-24`. Ainda pode haver DoS distribuído ou saturação externa. |
| Dispositivo aprovado malicioso/comprometido | Pode transmitir mídia/estado malicioso, abusar permissões de sessão ou manter acesso como dispositivo confiável. | Diálogo permite “sempre”, “só desta vez” ou recusar (`docs/protocol.md:74-78`; `crates/lc-server/src/events.rs:26-30`). Reautenticação com chave não prova que o cliente permaneça íntegro; existe revogação na lista, mas não isolamento de dispositivo comprometido (`crates/lc-server/src/trust/store.rs:191-232`). |
| Vazamento de QR/token e rotação | Quem obtiver QR/screenshot/tela pode parear enquanto token válido; logs ou cópia externa podem prolongar exposição. | Rotação pós-pareamento e ação “Regenerar” (`docs/protocol.md:47-48,77`; `crates/lc-server/src/trust/store.rs:176-182`). UI expõe URI para QR e mascara código exibido (`desktop/src-tauri/src/commands.rs:68-77,84-95`; `desktop/src-tauri/src/commands/network.rs:77-93`). Não há garantia contra captura de tela/foto; rotação não apaga cópias já obtidas. |
| Roubo da trust store/segredos em repouso | Token persistido permite tentativa de novo pareamento; hash de chave possibilita validação offline de hipótese se segredo conhecido. Roubo do celular pode expor chave armazenada. | PC armazena hash SHA-256 de `deviceKey`, mas token de pareamento é mantido como texto na store (`crates/lc-server/src/trust/store.rs:86-96,191-205`). Comparação em tempo constante existe (`crates/lc-server/src/trust/secrets.rs:31-40`). Criptografia em repouso/ACL específica não foi verificada; atomicidade de gravação não implica confidencialidade (`crates/lc-server/src/trust/atomic.rs:9-16`). |
| Acesso físico/ADB ao celular ou PC | Controle do endpoint pode expor credenciais, alterar software, aprovar atacante ou capturar mídia. | USB está especificado com `adb reverse` (`docs/protocol.md:37,84-86`) e a janela de aprovação USB de 15 s, restrita a loopback e de uso único, está implementada no servidor (`crates/lc-server/src/trust/usb_window.rs:1-9`); a integração ADB do desktop é da Fase 5 e não está implementada neste commit. Nada disso protege contra controle do próprio host/dispositivo. Segurança contra root/ADB autorizado/conta desktop comprometida não foi verificada. |
| Descoberta mDNS e periféricos virtuais (futuro) | Caminhos alternativos e endpoints locais podem ampliar a superfície quando implementados. | **Não implementados neste commit:** não há dependência/código mDNS, `adb reverse` nem câmera/microfone virtual (os indicadores correspondentes na UI estão marcados “em breve”; Fase 5 do plano). O protocolo §5 especifica que o TXT do anúncio não leva segredos, mas isso é requisito futuro, não garantia implementada; descoberta, quando existir, não será controle de acesso (`docs/protocol.md:51-61`). Revisão de driver/USB/câmera virtual fica para as fases correspondentes; não se afirma isolamento/proteção. |

## Aprovação: proteção e limites

A aprovação protege contra pareamento casual/não autorizado: só quem conhece token válido chega ao pedido, e operador do PC pode permitir sempre, permitir uma vez ou recusar. Ela **não** provê confidencialidade, integridade, disponibilidade ou autenticação criptográfica da identidade do dispositivo/PC. O nome exibido é dado informado pelo cliente, não prova identidade (`docs/protocol.md:74-78`; `crates/lc-server/src/handshake/perform.rs:60-109`). A aprovação de conexão não substitui segurança do endpoint nem impede MITM.

## Controles verificados

- Tamanhos: `random_hex(16)` gera token de 128 bits; `random_hex(32)` gera chave de 256 bits; token em 32 hex e chave em 64 hex (`crates/lc-server/src/trust/secrets.rs:10-21`; `crates/lc-server/src/trust/store.rs:95,203-205`).
- Token rotaciona após aprovação e sob comando de regeneração (`crates/lc-server/src/handshake/approval.rs:40-45`; `crates/lc-server/src/trust/store.rs:176-182`).
- Hash de `deviceKey` no PC: SHA-256; verificação transforma chave apresentada em hash (`crates/lc-server/src/trust/store.rs:191-205,224-233`).
- Comparação em tempo constante implementada com `subtle::ConstantTimeEq` (`crates/lc-server/src/trust/secrets.rs:7,31-40`), usada em token, hash de dispositivo e ticket de áudio (`crates/lc-server/src/trust/store.rs:185-188,232`; `crates/lc-server/src/audio/tickets.rs:82-96`).
- Limites e aprovação: `max_unauth_conns=8`, `hello_timeout=5s`, aprovação `30s`, auth limit `5/60s` e bloqueio `300s`, conexão `20/10s` (`crates/lc-server/src/config.rs:10-24`; padrões aplicados em `desktop/src-tauri/src/server.rs:46-54`).
- Mensagens da rede expõem códigos de motivo e não exibem frase arbitrária da rede na UI, mas códigos distinguem causas (`docs/protocol.md:119`).
- Redação: `Debug` de trust store/resultado e tickets omite segredos (`crates/lc-server/src/trust/store.rs:66-72,276-284`; `crates/lc-server/src/handshake.rs:89-111`; `crates/lc-server/src/audio/tickets.rs:27-39,132`). Logs inspecionados na autenticação registram motivo/ID sem token tentado (`crates/lc-server/src/handshake/perform.rs:67-71`; `crates/lc-server/src/trust/ratelimit.rs:1-8`). Isso não demonstra ausência de segredo em todo log/telemetria do sistema.

## Riscos residuais e recomendações priorizadas

1. Usar somente em LAN considerada confiável; não encaminhar/expor a porta TCP à Internet.
2. Aplicar firewall do host para limitar origens e manter rede/host atualizados.
3. Proteger QR/token contra fotografias, screenshots, compartilhamento e logs; regenerar imediatamente após suspeita de vazamento.
4. Futuramente avaliar TLS/DTLS ou mecanismo equivalente de confidencialidade, integridade e autenticação mútua, com gestão de identidade/chaves. Não implementado neste escopo.
5. Revisar limitadores com cenários de múltiplas origens, IPv6 e tráfego de conexão lenta; os valores atuais não são garantia anti-DoS.
6. Proteger o PC e o armazenamento privado Android, revogar dispositivos não reconhecidos e não usar build de debug em ambiente exposto.
7. Fazer avaliação separada de USB/ADB, descoberta mDNS e câmeras/microfone virtuais; não inferir garantias de isolamento.

## Auditorias de dependências

- **Data:** 2026-11-16.
- **Ferramentas:** `cargo-audit 0.23.0`, `cargo-deny 0.21.0`; instaladas com `cargo install cargo-audit --locked` e `cargo install cargo-deny --locked` (ambos concluídos).
- **Ambiente:** `cargo 1.99.1 (5f94df478 2026-08-27)`; `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.

| Comando | Exit code | Resultado |
|---|---:|---|
| `cargo audit` | 0 | Carregou 1509 advisories. 5 advisories apresentados: `RUSTSEC-2024-0370` (proc-macro-error, não mantido), `RUSTSEC-2026-0371` (zbus_names, não mantido), `RUSTSEC-2026-0370` (zvariant, não mantido), `RUSTSEC-2026-0372` (zvariant_derive, não mantido) e `RUSTSEC-2024-0429` (glib, unsoundness em iteradores; corrigido em >=0.20.0). Não foram exibidos níveis CVSS/severidade; não atribuí severidade adicional. Também listou `async-executor 1.14.0` como yanked, sem ID indicado. Saída final: 6 allowed warnings; o comando terminou com sucesso, portanto sucesso não significa ausência de advisories. |
| `cargo deny check licenses` | 4 | `deny.toml` não existe na raiz. O comando foi executado, mas recusou licenças que não estão explicitamente permitidas (incluindo Unicode-3.0 e MIT); **bloqueado por falta de configuração/decisão de política, não aprovado**. Nenhum `deny.toml` foi criado e nenhuma licença foi aprovada nesta tarefa. |

Limitações: auditoria baseada no lockfile e no banco local de advisories consultado por `cargo-audit`; o resultado é temporal e não substitui revisão de aplicabilidade/exposição. Nenhuma dependência foi corrigida nesta tarefa. O `cargo deny` não fornece uma validação de licenças aprovada enquanto não houver `deny.toml` definido por decisão apropriada.

## Limitações e observações

Esta é revisão estática do commit informado; não foram executados testes dinâmicos, captura de rede, fuzzing, nem testes em Android/Windows/USB. Foi executado `cargo audit`, conforme resultados acima; checagem de licenças permanece bloqueada por ausência de configuração. Não se avaliou configuração efetiva de firewall, permissões do sistema de arquivos ou proteção em repouso fornecida pelo SO.

O CHEATSHEET e protocolo descrevem o token/chave e controles; o protocolo é fonte normativa (`docs/protocol.md:1`). A configuração documentada indica porta padrão e limites; valores efetivos podem ser configuráveis. A tarefa pediu verificar bind em `0.0.0.0`; confirmei em `desktop/src-tauri/src/server.rs:41`. O áudio tem ticket de uso único e validade de 10 s (`docs/protocol.md:34`), mas isso não cifra o canal.
