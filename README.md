# LocalCamera

Transforma um celular Android em webcam e microfone para o Windows 11, via Wi-Fi ou USB.

## Desenvolvimento

```powershell
pnpm install
pnpm run ci
```

(Os comandos completos da CI chegam nas fases seguintes.)

### Testes E2E (Playwright)

O Playwright inicia seus próprios servidores Vite com `--strictPort` e não
reutiliza servidores existentes. Por padrão, usa `1421` (desktop) e `5199`
(mobile), isoladas das portas comuns de desenvolvimento. Se uma delas estiver
ocupada, escolha duas portas TCP livres e passe-as ao comando:

```powershell
$env:LC_E2E_DESKTOP_PORT = "1421"
$env:LC_E2E_MOBILE_PORT = "5199"
pnpm e2e
Remove-Item Env:LC_E2E_DESKTOP_PORT, Env:LC_E2E_MOBILE_PORT
```

Substitua os exemplos por portas livres no ambiente; os mesmos valores são
usados pelos servidores e pelos testes.
