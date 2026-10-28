# ADR 0003 — Canal Frame Server ↔ app: pipe nomeado
- **Data:** 2026-10-28
- **Status:** aceito

## Contexto

O serviço Frame Server do Windows (`svchost.exe -k Camera`, conta
`NT AUTHORITY\LocalService`, sessão 0) carrega a media source da câmera
virtual (DLL em `C:\Program Files\LocalCameraSpike\`). A DLL precisa enviar
quadros NV12 1080p30 para o "app" (usuário comum, sessão 1) que consome a
câmera. O canal deve funcionar entre sessões (0 ↔ 1) e ser seguro (apenas o
usuário que instalou + o Frame Server podem acessar).

O spike S2 testou duas opções:
1. **Named pipe** (`\\.\pipe\LocalCameraSpike`) criado pelo app, consumido pela DLL.
2. **Memória compartilhada** (`CreateFileMappingW` em `Global\`) criada pelo app, consumida pela DLL.

## Decisão

**Named pipe** é o canal Frame Server ↔ app. Justificativas:

- **Entre sessões**: named pipes são objetos máquina-wide, fora dos
  namespaces `Global\`/`Local\` por sessão; a DLL (sessão 0) consegue abrir
  o pipe criado pelo usuário (sessão 1) desde que a DACL permita.
- **Criação sem privilégio**: usuário comum cria o named pipe sem elevação.
  Em contraste, criar um file mapping em `Global\` é negado para usuário
  comum (`ERROR_ACCESS_DENIED`, 0x80070005) — exigiria `SeCreateGlobalPrivilege`
  que o usuário não tem.
- **DACL restritiva**: o pipe é criado com DACL que concede acesso total
  (`0x1F01FF`) apenas ao SID do usuário atual e a `NT AUTHORITY\LocalService`
  (S-1-5-19). Qualquer outro usuário recebe `ERROR_ACCESS_DENIED` (5) —
  verificado empiricamente com uma conta local temporária (`NEGADO 5`).
- **Rejeita remoto**: `PIPE_REJECT_REMOTE_CLIENTS` impede conexões de outras
  máquinas na rede (garantia de código em `spikes/s2/pipe-server/src/sec.rs`).
- **Desempenho medido**: 87,0 MB/s em 1500 s, 41.978 quadros, 0 perdas,
  latência p50 1,87 ms / p95 2,95 ms. 21 reconexões (20× abrir/fechar)
  sem falha.

A alternativa **shm em `Global\`** é descartada: usuário comum não pode
criar o objeto. A reversão de papéis (DLL cria `Global\`, app apenas abre)
é viável porque a DLL roda como LocalService e tem
`SeCreateGlobalPrivilege`, mas adiciona complexidade sem benefício claro
sobre o pipe.

## Consequências

- **Fase 4:** o cliente do canal (`crates/lc-client` ou equivalente) abre
  `\\.\pipe\LocalCameraSpike` e consome quadros NV12; a media source do
  produto escreve no mesmo pipe.
- **Instalador:** cria o pipe com DACL fixa em 2 SIDs (usuário atual +
  LocalService). Não há `regsvr32` — registro via CLSID no `HKLM`, como o
  S1.
- **Validar na Fase 4:** outro usuário do mesmo PC não consegue acessar o
  pipe; outro PC na rede não consegue conectar (`PIPE_REJECT_REMOTE_CLIENTS`).
- **Spike S2 desinstalado** após o veredito: remover DLL, CLSID, pasta
  `C:\Program Files\LocalCameraSpike`, chave `HKLM\SOFTWARE\LocalCameraSpike`
  e usuário temporário de teste (requer elevação — `spikes/s1/uninstall.ps1`
  cobre câmera + CLSID + pasta; a chave `LocalCameraSpike` e o usuário de
  teste precisam de remoção manual elevada).

## Alternativas consideradas

- **Memória compartilhada em `Global\`:** descartado — `CreateFileMappingW`
  em `Global\` retorna `ERROR_ACCESS_DENIED` para usuário comum.
- **Reversão de papéis (DLL cria `Global\`, app abre):** viável tecnicamente
  (DLL tem `SeCreateGlobalPrivilege` como LocalService), mas mais complexo
  que o pipe e sem vantagem de desempenho relevante.
