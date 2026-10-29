# mock-sender

Finge ser o celular LocalCamera: pareia pela URI, reconecta com a chave
salva, envia vídeo H.264 (Annex-B) com *pacing* e áudio (seno 440 Hz).

## Flags

| Flag | Padrão | Descrição |
|---|---|---|
| `--uri` | — | URI de pareamento (`localcamera://connect?...`); conflita com `--host`/`--token` |
| `--host` | — | Host do PC (com `--token` ou um estado já pareado) |
| `--port` | `48650` | Porta do servidor (ignorada quando a URI traz a sua) |
| `--token` | — | Token de pareamento (32 hex) |
| `--device-id` | do estado ou UUID | ID do dispositivo enviado no `HELLO` |
| `--name` | `Mock Phone` | Nome exibido do celular |
| `--state-file` | — | Arquivo `{deviceId, deviceKey}`; salvo ao parear e reutilizado |
| `--device-key` | — | Chave já pareada (dispensa `--token`) |
| `--file` | — | Arquivo `.h264` (Annex-B) |
| `--loop` | desligado | Reinicia o vídeo ao acabar o arquivo |
| `--fps` | `30` | Quadros por segundo do envio |
| `--audio` | desligado | Abre o socket de áudio e envia o seno |
| `--tone` | `440` | Frequência do seno (Hz) |
| `--seconds` | — | Duração; sem valor, até o arquivo acabar ou Ctrl+C |
| `--fragment N` | — | Todos os `write` saem em pedaços de N bytes (flush entre eles) |
| `--bad-magic` | desligado | Primeiro quadro com `magic` errado |
| `--huge-length` | desligado | Primeiro quadro com `length = 8 MiB + 1` (só o cabeçalho) |
| `--bad-json` | desligado | `HELLO` com JSON inválido |
| `--unknown-type` | desligado | Quadro de tipo `0x99` a cada 30 quadros de vídeo |
| `--delay-hello MS` | — | Espera MS ms antes do `HELLO` |
| `--no-pong` | desligado | Não responde `PING` |
| `--drop-ping` | desligado | Descarta o 1º `PING` e responde os demais |
| `--slow KBPS` | — | Limita a vazão de escrita a KBPS kbit/s |
| `--stall MS@SEG` | — | Depois de SEG s, para de escrever por MS ms |
| `--kill-after SEG` | — | Fecha o socket abruptamente (sem `BYE`) depois de SEG s |
| `--reconnect N` | — | Conecta/envia 1 s/desconecta, N vezes (usa `--state-file`) |
| `--fake-version M.m` | `1.0` | Anuncia `protocol {major: M, minor: m}` no `HELLO` |

## Exemplos

```powershell
# Parear com a URI do QR (grava o estado para a próxima vez)
cargo run -q -p mock-sender -- --uri "localcamera://connect?v=1&hosts=192.168.0.10&port=48650&token=0123456789abcdef0123456789abcdef&pc=pc-1&name=Mock" --state-file mock-state.json

# Reconectar com a chave salva
cargo run -q -p mock-sender -- --host 127.0.0.1 --state-file mock-state.json

# Só áudio por 10 s
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --audio --seconds 10

# Vídeo em loop por 30 s
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --file fixtures/hd30.h264 --loop --seconds 30

# Falhas determinísticas (exemplos)
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --huge-length
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --no-pong --seconds 5
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --fragment 1 --file fixtures/pattern43.h264
cargo run -q -p mock-sender -- --host 127.0.0.1 --token 0123456789abcdef0123456789abcdef --slow 400 --file fixtures/hd30.h264 --seconds 10
```

O mock termina com `0` (ok), `2` (recusado pelo servidor) ou `1` (erro).
