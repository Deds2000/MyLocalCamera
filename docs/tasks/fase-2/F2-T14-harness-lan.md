# F2-T14 — Harness LAN seguro (IT-2.05/IT-2.06) — runbook

> Arquivos: `crates/lc-server/examples/it_lan_harness.rs` (servidor de teste) +
> `it_lan_harness/pairing_broker.rs` (entrega efêmera da URI) + este documento.
> O harness substitui `soak_server_lan`, que imprime token no stdout.

## O que é / o que não é
- **É**: servidor de teste LAN com estado isolado por execução, porta efêmera,
  URI/token apenas em memória e entrega por broker HTTP one-shot, sem arquivo,
  stdout, argv ou log; auto-aprovação via `Decision`, stats não sensíveis no
  console, watchdog (`--seconds`) e cleanup limitado ao estado da execução.
- **Não é**: teste do app. O caminho real do celular (câmera → encoder →
  serviço → rede) é quem gera a evidência de que "vídeo real chega ao
  servidor" (IT-2.05). A auto-aprovação aqui **não** prova a UI de aprovação
  (desktop, Fase 3); a sessão sem UI **não** prova UI/serviço.

## Segurança (o que o harness garante vs. o que o operador garante)
| Item | Tratamento |
|---|---|
| Token/URI de pareamento | Só em memória; entregue uma vez por HTTP na LAN ao teste instrumentado; nunca em stdout/argv/log/arquivo |
| Estado do PC | Diretório novo e único `%TEMP%\localcamera-it-<pid>-<ts>` (ou `--state`); nunca reutiliza/apaga estado alheio; o `trust.json` fica dentro desse diretório |
| Bind | IP de LAN detectado (privado); `--all` (0.0.0.0) só explícito |
| Porta | Efêmera (`0`) por padrão; `--port N` fixa (necessária no IT-2.06) |
| Watchdog | `--seconds N` (embutido) + externo (abaixo), sempre sobre o PID exato iniciado |
| Cleanup | Somente o diretório criado pela execução (`--keep` mantém; `--state` nunca apaga) |
| Transporte da URI para o celular | O teste Android faz GET ao endpoint não secreto `http://<ip>:<porta>/pairing` e mantém o corpo só em memória; usar rede privada confiável e um único celular de teste |

## Pré-requisitos
1. Celular e PC no mesmo Wi-Fi (sem isolamento de clientes); IP LAN do PC
   visível em `ipconfig` (o harness imprime o IP detectado — conferir).
2. APK debug **corrente** no aparelho (com `SessionModePicker` — o build
   validado pela sessão principal; não rebuildar em paralelo com outros agentes).
3. Primeiro uso: o firewall do Windows pode bloquear o executável. A IA não deve pedir ao usuário
   para confirmar um diálogo nem abrir regra ampla: só aplicar uma regra mínima para o executável/
   porta e perfil privado se o cartão autorizar e a permissão disponível permitir. Se exigir UAC ou
   intervenção manual, parar e registrar bloqueio técnico; não desativar o firewall nem usar perfil público.
4. Compilar (apenas quando não houver outro agente compilando):
   `cargo build -p lc-server --example it_lan_harness`.

## IT-2.05 — vídeo real chega ao servidor (por aparelho)
1. `cargo run -p lc-server --example it_lan_harness -- --seconds 300`
2. Console imprime (sem segredos): `estado: <dir>`, `bind: <ip>:<port>`,
   `pc: <pcId>` e endereço do broker `http://<ip>:<porta>/pairing`.
3. Instale o APK de teste e execute a classe com o endereço **não secreto** do
   broker. O teste busca a URI em memória e a entrega a
   `BridgeActivity.onNewIntent` diretamente, sem passar o segredo ao
   `ActivityTaskManager`; não imprime nem persiste a URI/token:
   ```powershell
   adb -s RXCT800A2HR install -r .\mobile\android\app\build\outputs\apk\androidTest\debug\app-debug-androidTest.apk
   adb -s RXCT800A2HR shell am instrument -w -e class com.localcamera.app.DeepLinkDispatchTest -e pairingBroker <ip>:<porta> -e keepPairing true -e holdStreamingMs 12000 com.localcamera.app.test/androidx.test.runner.AndroidJUnitRunner
   ```
   O teste espera `appUrlOpen` e `deviceKey` após WELCOME, examina o logcat em
   memória e só retorna pass/fail. O modo fictício sem `pairingBroker` cobre o
   lançamento Android externo com valor sem segredo. Não use `adb am start -d`,
   copie a URI/token ou arquive log bruto. Execute numa rede privada confiável
   com um único celular.
4. Esperado no console: `pareamento: <deviceId> (<nome>)` →
   `aprovacao: aprovado (sempre)` → `sessao iniciada: …` → `stats` a cada 10 s.
5. Evidência: `stats` finais (fps/kbps/frames = **recebimento no servidor**),
   screenshot "Transmitindo" no celular e resolução efetiva
   (Moto: 720p esperado com o seletor de modo; Samsung: 1080p — conferir
   `STREAM_CONFIG` no log do servidor).
6. Encerrar (Ctrl+C ou `--seconds`); o estado é removido automaticamente.
7. Repetir para o segundo aparelho.
8. Se aparecer `bad_token`: o token rotacionou após um pareamento anterior —
   refazer a fase 1 com estado novo (ou a reconexão já funciona por `deviceKey`).

## IT-2.06 — reconectar com a chave, sem nova aprovação
1. **Fase 1** (parear; porta fixa para o celular reaproveitar):
   `cargo run -p lc-server --example it_lan_harness -- --port 48651 --keep --seconds 180`
   → executar `DeepLinkDispatchTest` com o broker e `-e keepPairing true` → o
   celular pareia e **salva a `deviceKey` no Keystore real** → encerrar
   (`--keep` mantém o diretório; o console imprime o caminho).
2. **Fase 2** (reconectar):
   `cargo run -p lc-server --example it_lan_harness -- --state <dir-da-fase-1> --port 48651 --seconds 180`
3. No celular, reconectar **sem novo token**: Home → Reconectar (mesmos
   IPs/porta — a `deviceKey` do Keystore tem prioridade sobre o token da URI
   — `HelloBuilder`).
4. **Critério**: o servidor **não** emite `pareamento:`/`aprovacao:` — vai
   direto a `sessao iniciada`; o celular não mostra "Aguardando aprovação".
5. O estado fica mantido (o operador remove o `<dir>` impresso ao final —
   contém apenas estado de teste, nunca o estado do app).

## Dois aparelhos ao mesmo tempo
Servidor = **uma** sessão (o segundo aparelho recebe `busy`). Duas instâncias:
diretórios próprios (automáticos) e `--port` distinto por instância
(ex.: 48651/48652); cada URI é colada no aparelho correspondente.

## Armazenamento real × fake (não misturar evidências)
| Onde a chave vive | Comprova |
|---|---|
| `AndroidKeyStore` do celular (`KeystoreSecureStore`) — usado no IT-2.06 | armazenamento de **produção** da `deviceKey` |
| `mock-sender --state-file` (JSON em texto claro no PC) | só o protocolo (fluxo de reconexão) — **não** o Keystore |
| Fake `SecureStore` nos unitários JVM (UT-2.07) | só a lógica do `PairingStore` |
| `trust.json` no diretório isolado do harness | loja de confiança do **servidor** (código real), em estado de teste, removido ao fim |

## Watchdog externo (somente o PID exato iniciado)
```powershell
$exe = Join-Path (Get-Location) 'target\debug\it_lan_harness.exe'
$p = Start-Process -PassThru $exe -ArgumentList '--seconds','300','--port','48651'
Start-Sleep -Seconds 330
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
```
Nunca matar por nome ou por outro PID (regra permanente: não matar os ffmpeg
do usuário). O `--seconds` embutido já encerra sozinho; o externo é rede de segurança.

## Cleanup pós-crash
O console imprime `estado: <dir>` no início. Se a execução morreu sem cleanup,
remover **somente** aquele diretório exato (contém o estado de teste desta
execução; URI/token não são gravados nesse diretório).

## Limites (não declarar sucesso sem medir)
- fps/kbps do console = **recebimento no servidor**; p95 câmera→PC, 30 fps
  sustentado e 10 min **não** são comprovados aqui (exigem medição com
  timestamps; 8 Mbps é referência, não mínimo garantido).
- Auto-aprovação não prova a UI de aprovação; a sessão sem UI não prova
  UI/serviço (Fase 3).
- Após a alteração do broker em memória, recompilar e executar com o aparelho;
  registrar no relatório modo, fps/kbps recebidos e firewall.

## Operação segura
- O broker aceita somente `GET /pairing`, entrega uma vez e encerra. Não habilite
  o harness numa rede pública; execute o androidTest logo após iniciar o servidor.
- `pairingBroker` contém apenas IP/porta não secretos; URI/token não passam por
  argumentos do ADB, clipboard, arquivo ou saída do runner.
- O `DeepLinkDispatchTest` inspeciona logcat em memória. Com broker, ele também
  confirma `deviceKey` no Keystore após WELCOME; por padrão remove o PC de teste e
  para o serviço. `-e keepPairing true` preserva PC/chave para IT-2.06.

## Pendências operacionais
1. Portas fixas do IT-2.06 (48651/48652 propostas) — a IA confirma disponibilidade antes de iniciar.
2. Escolher APK debug corrente (com seletor de modo); evitar rebuild em paralelo.
3. Firewall Windows: seguir a regra de segurança acima; se exigir interação/UAC, registrar bloqueio sem delegar ao usuário.
